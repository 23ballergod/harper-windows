//! Quantized Qwen3 model, adapted from candle-transformers 0.11 `quantized_qwen3` (MIT OR
//! Apache-2.0, https://github.com/huggingface/candle).
//!
//! Changes from upstream: attention always uses a plain key/value cache so it can be saved,
//! restored and truncated (the fixed instruction prompt is processed once, and rejected draft
//! tokens are dropped), and [`ModelWeights::forward_all`] returns logits for every position.

use std::io::{Read, Seek};

use candle_core::quantized::{QMatMul, QTensor, gguf_file};
use candle_core::{DType, Device, Result, Tensor};
use candle_nn::{Activation, Embedding, Module};
use candle_transformers::{quantized_nn::RmsNorm, utils::repeat_kv};

struct Gguf<'a, R: Read + Seek> {
    ct: gguf_file::Content,
    reader: &'a mut R,
    device: Device,
}

impl<R: Read + Seek> Gguf<'_, R> {
    fn qmatmul(&mut self, name: &str) -> Result<QMatMul> {
        let ws = self.ct.tensor(self.reader, name, &self.device)?;
        QMatMul::from_qtensor(ws)
    }

    fn rms_norm(&mut self, name: &str, eps: f64) -> Result<RmsNorm> {
        let ws = self.ct.tensor(self.reader, name, &self.device)?;
        RmsNorm::from_qtensor(ws, eps)
    }

    fn tensor(&mut self, name: &str) -> Result<QTensor> {
        self.ct.tensor(self.reader, name, &self.device)
    }

    fn u32(&self, key: &str) -> Result<usize> {
        match self.ct.metadata.get(key) {
            Some(v) => Ok(v.to_u32()? as usize),
            None => candle_core::bail!("cannot find {key} in metadata"),
        }
    }

    fn f32(&self, key: &str) -> Result<f64> {
        match self.ct.metadata.get(key) {
            Some(v) => Ok(f64::from(v.to_f32()?)),
            None => candle_core::bail!("cannot find {key} in metadata"),
        }
    }
}

#[derive(Debug, Clone)]
struct Mlp {
    gate_proj: QMatMul,
    up_proj: QMatMul,
    down_proj: QMatMul,
}

impl Module for Mlp {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let gate = self.gate_proj.forward(x)?.apply(&Activation::Silu)?;
        let up = self.up_proj.forward(x)?;
        self.down_proj.forward(&(gate * up)?)
    }
}

#[derive(Debug, Clone)]
struct RotaryEmbedding {
    sin: Tensor,
    cos: Tensor,
}

impl RotaryEmbedding {
    fn new(head_dim: usize, max_len: usize, theta: f64, dev: &Device) -> Result<Self> {
        let inv_freq: Vec<_> = (0..head_dim)
            .step_by(2)
            .map(|i| 1f32 / theta.powf(i as f64 / head_dim as f64) as f32)
            .collect();
        let len = inv_freq.len();
        let inv_freq = Tensor::from_vec(inv_freq, (1, len), dev)?;
        let t = Tensor::arange(0u32, max_len as u32, dev)?
            .to_dtype(DType::F32)?
            .reshape((max_len, 1))?;
        let freqs = t.matmul(&inv_freq)?;
        Ok(Self {
            sin: freqs.sin()?,
            cos: freqs.cos()?,
        })
    }

    fn apply(&self, q: &Tensor, k: &Tensor, offset: usize) -> Result<(Tensor, Tensor)> {
        let (_, _, seq_len, _) = q.dims4()?;
        let cos = self.cos.narrow(0, offset, seq_len)?;
        let sin = self.sin.narrow(0, offset, seq_len)?;
        Ok((
            candle_nn::rotary_emb::rope(&q.contiguous()?, &cos, &sin)?,
            candle_nn::rotary_emb::rope(&k.contiguous()?, &cos, &sin)?,
        ))
    }
}

#[derive(Debug, Clone)]
struct Layer {
    q_proj: QMatMul,
    k_proj: QMatMul,
    v_proj: QMatMul,
    o_proj: QMatMul,
    q_norm: RmsNorm,
    k_norm: RmsNorm,
    ln1: RmsNorm,
    ln2: RmsNorm,
    mlp: Mlp,
    num_heads: usize,
    num_kv_heads: usize,
    head_dim: usize,
    kv_cache: Option<(Tensor, Tensor)>,
}

impl Layer {
    fn attention(
        &mut self,
        x: &Tensor,
        mask: Option<&Tensor>,
        offset: usize,
        rotary: &RotaryEmbedding,
    ) -> Result<Tensor> {
        let (b, l, _) = x.dims3()?;
        let q = self
            .q_proj
            .forward(x)?
            .reshape((b, l, self.num_heads, self.head_dim))?
            .transpose(1, 2)?;
        let k = self
            .k_proj
            .forward(x)?
            .reshape((b, l, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let v = self
            .v_proj
            .forward(x)?
            .reshape((b, l, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;

        // Qwen3 normalizes each head's queries and keys.
        let q = self.q_norm.forward(&q.flatten(0, 2)?)?.reshape((
            b,
            self.num_heads,
            l,
            self.head_dim,
        ))?;
        let k = self.k_norm.forward(&k.flatten(0, 2)?)?.reshape((
            b,
            self.num_kv_heads,
            l,
            self.head_dim,
        ))?;

        let (q, k) = rotary.apply(&q, &k, offset)?;

        let (k, v) = match &self.kv_cache {
            Some((pk, pv)) => (Tensor::cat(&[pk, &k], 2)?, Tensor::cat(&[pv, &v], 2)?),
            None => (k, v.contiguous()?),
        };
        self.kv_cache = Some((k.clone(), v.clone()));

        let groups = self.num_heads / self.num_kv_heads;
        let k = repeat_kv(k, groups)?.contiguous()?;
        let v = repeat_kv(v, groups)?.contiguous()?;

        let scale = 1.0 / (self.head_dim as f64).sqrt();
        let mut scores = (q.matmul(&k.transpose(2, 3)?)? * scale)?;
        if let Some(mask) = mask {
            scores = scores.broadcast_add(mask)?;
        }
        let probs = candle_nn::ops::softmax_last_dim(&scores)?;
        let ctx =
            probs
                .matmul(&v)?
                .transpose(1, 2)?
                .reshape((b, l, self.num_heads * self.head_dim))?;
        self.o_proj.forward(&ctx)
    }

    fn forward(
        &mut self,
        x: &Tensor,
        mask: Option<&Tensor>,
        offset: usize,
        rotary: &RotaryEmbedding,
    ) -> Result<Tensor> {
        let h = self.attention(&self.ln1.forward(x)?, mask, offset, rotary)?;
        let x = (x + h)?;
        let h = self.mlp.forward(&self.ln2.forward(&x)?)?;
        x + h
    }
}

#[derive(Debug, Clone)]
pub struct ModelWeights {
    embed_tokens: Embedding,
    layers: Vec<Layer>,
    norm: RmsNorm,
    lm_head: QMatMul,
    rotary: RotaryEmbedding,
    device: Device,
}

impl ModelWeights {
    pub fn from_gguf<R: Read + Seek>(
        ct: gguf_file::Content,
        reader: &mut R,
        device: &Device,
    ) -> Result<Self> {
        let mut gg = Gguf {
            ct,
            reader,
            device: device.clone(),
        };
        let num_heads = gg.u32("qwen3.attention.head_count")?;
        let num_kv_heads = gg.u32("qwen3.attention.head_count_kv")?;
        let head_dim = gg.u32("qwen3.attention.key_length")?;
        let num_layers = gg.u32("qwen3.block_count")?;
        let hidden_size = gg.u32("qwen3.embedding_length")?;
        // Sentences are short; there is no need to precompute positions for the full context.
        let max_len = gg.u32("qwen3.context_length")?.min(8192);
        let eps = gg.f32("qwen3.attention.layer_norm_rms_epsilon")?;
        let rope_theta = gg.f32("qwen3.rope.freq_base")?;

        let embed = gg.tensor("token_embd.weight")?;
        let embed_tokens = Embedding::new(embed.dequantize(device)?, hidden_size);
        let rotary = RotaryEmbedding::new(head_dim, max_len, rope_theta, device)?;

        let mut layers = Vec::with_capacity(num_layers);
        for i in 0..num_layers {
            let p = format!("blk.{i}");
            layers.push(Layer {
                q_proj: gg.qmatmul(&format!("{p}.attn_q.weight"))?,
                k_proj: gg.qmatmul(&format!("{p}.attn_k.weight"))?,
                v_proj: gg.qmatmul(&format!("{p}.attn_v.weight"))?,
                o_proj: gg.qmatmul(&format!("{p}.attn_output.weight"))?,
                q_norm: gg.rms_norm(&format!("{p}.attn_q_norm.weight"), eps)?,
                k_norm: gg.rms_norm(&format!("{p}.attn_k_norm.weight"), eps)?,
                ln1: gg.rms_norm(&format!("{p}.attn_norm.weight"), eps)?,
                ln2: gg.rms_norm(&format!("{p}.ffn_norm.weight"), eps)?,
                mlp: Mlp {
                    gate_proj: gg.qmatmul(&format!("{p}.ffn_gate.weight"))?,
                    up_proj: gg.qmatmul(&format!("{p}.ffn_up.weight"))?,
                    down_proj: gg.qmatmul(&format!("{p}.ffn_down.weight"))?,
                },
                num_heads,
                num_kv_heads,
                head_dim,
                kv_cache: None,
            });
        }

        let norm = gg.rms_norm("output_norm.weight", eps)?;
        // Smaller Qwen3 models tie the output projection to the embeddings.
        let lm_head = match gg.tensor("output.weight") {
            Ok(t) => t,
            Err(_) => gg.tensor("token_embd.weight")?,
        };
        Ok(Self {
            embed_tokens,
            layers,
            norm,
            lm_head: QMatMul::from_qtensor(lm_head)?,
            rotary,
            device: device.clone(),
        })
    }

    fn mask(&self, len: usize, offset: usize) -> Result<Tensor> {
        let mask: Vec<f32> = (0..len)
            .flat_map(|i| {
                (0..len + offset).map(move |j| {
                    if j <= i + offset {
                        0.
                    } else {
                        f32::NEG_INFINITY
                    }
                })
            })
            .collect();
        Tensor::from_slice(&mask, (1, 1, len, len + offset), &self.device)
    }

    fn hidden_states(&mut self, input: &Tensor, offset: usize) -> Result<Tensor> {
        let (_, len) = input.dims2()?;
        let mask = (len > 1).then(|| self.mask(len, offset)).transpose()?;
        let mut h = self.embed_tokens.forward(input)?;
        for layer in &mut self.layers {
            h = layer.forward(&h, mask.as_ref(), offset, &self.rotary)?;
        }
        self.norm.forward(&h)
    }

    /// Logits for the last position only.
    pub fn forward(&mut self, input: &Tensor, offset: usize) -> Result<Tensor> {
        let h = self.hidden_states(input, offset)?;
        let len = h.dim(1)?;
        self.lm_head.forward(&h.narrow(1, len - 1, 1)?)?.squeeze(1)
    }

    /// Logits for every input position, shaped `(len, vocab)`.
    pub fn forward_all(&mut self, input: &Tensor, offset: usize) -> Result<Tensor> {
        let h = self.hidden_states(input, offset)?;
        self.lm_head.forward(&h)?.squeeze(0)
    }

    pub fn clear_kv_cache(&mut self) {
        for layer in &mut self.layers {
            layer.kv_cache = None;
        }
    }

    pub fn snapshot_kv_cache(&self) -> Vec<Option<(Tensor, Tensor)>> {
        self.layers.iter().map(|l| l.kv_cache.clone()).collect()
    }

    pub fn restore_kv_cache(&mut self, snapshot: &[Option<(Tensor, Tensor)>]) {
        for (layer, cache) in self.layers.iter_mut().zip(snapshot) {
            layer.kv_cache = cache.clone();
        }
    }

    /// Drops cached positions at and after `len`.
    pub fn truncate_kv_cache(&mut self, len: usize) -> Result<()> {
        for layer in &mut self.layers {
            if let Some((k, v)) = &layer.kv_cache {
                let keep = len.min(k.dim(2)?);
                layer.kv_cache = Some((k.narrow(2, 0, keep)?, v.narrow(2, 0, keep)?));
            }
        }
        Ok(())
    }
}
