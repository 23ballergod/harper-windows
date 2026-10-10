"""Run a HF seq2seq GEC model over a set. Usage: run_gec.py <hf_id> <name> <set> [--after harper]"""
import json, sys, time, os, torch
from transformers import AutoTokenizer, AutoModelForSeq2SeqLM
from huggingface_hub import HfApi
PROMPTS = {"grammarly/coedit-large": "Fix grammatical errors in this text: ", "grammarly/coedit-xl": "Fix grammatical errors in this text: ",
           "vennify/t5-base-grammar-correction": "grammar: ", "pszemraj/flan-t5-large-grammar-synthesis": ""}
mid, name, st = sys.argv[1:4]; after = "--after" in sys.argv
torch.set_num_threads(os.cpu_count())
D = os.path.dirname(os.path.abspath(__file__))
rows = [json.loads(l) for l in open(f"{D}/data/{st}.jsonl")]
pre = {r["id"]: r["output"] for r in map(json.loads, open(f"{D}/data/harper.{st}.jsonl"))} if after else {}
tok = AutoTokenizer.from_pretrained(mid); m = AutoModelForSeq2SeqLM.from_pretrained(mid, torch_dtype=torch.float32).eval()
if not after:
    info = HfApi().model_info(mid, files_metadata=True)
    size = sum(s.size or 0 for s in info.siblings if s.rfilename.endswith((".safetensors",)) ) or sum(s.size or 0 for s in info.siblings if s.rfilename.endswith(".bin"))
    lic = [t for t in (info.tags or []) if t.startswith("license")]
    json.dump({"id": mid, "bytes": size, "license": lic, "params": sum(p.numel() for p in m.parameters()), "cpus": os.cpu_count()}, open(f"{D}/predictions/{name}.info.json", "w"))
name += "+after-harper" if after else ""
with open(f"{D}/predictions/{name}.{st}.jsonl", "w") as f, torch.inference_mode():
    for r in rows:
        text = pre.get(r["id"], r["source"])
        t0 = time.perf_counter()
        ids = tok(PROMPTS[mid] + text, return_tensors="pt").input_ids
        out = tok.decode(m.generate(ids, max_new_tokens=min(256, 2 * ids.shape[1] + 20), num_beams=1, do_sample=False)[0], skip_special_tokens=True).strip()
        f.write(json.dumps({"id": r["id"], "output": out, "ms": (time.perf_counter() - t0) * 1000}) + "\n")
