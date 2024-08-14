#!/usr/bin/env python3

from datasets import load_dataset
dataset = load_dataset("lmsys/toxic-chat", "toxicchat0124")

for split, split_dataset in dataset.items():
    split_dataset.to_parquet(f"datasets/lmsys-toxic-chat_toxicchat0124-{split}.parquet")
