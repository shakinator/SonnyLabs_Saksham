#!/usr/bin/env python

from transformers import DistilBertTokenizerFast, DistilBertModel
import torch

torch.set_num_threads(1)
torch.device("cpu")

tokenizer = DistilBertTokenizerFast.from_pretrained('distilbert-base-uncased')
model = DistilBertModel.from_pretrained('distilbert-base-uncased')

print("quantizing...")
model = torch.quantization.quantize_dynamic(model, {torch.nn.Linear}, dtype=torch.qint8)
print("quantizing... done")

with torch.no_grad():
    # warm up
    txt = ''.join([f"Hello, my dog is cute {i}" for i in range(20)])
    inputs = tokenizer(txt, return_tensors="pt")
    print(inputs)
    print(inputs.input_ids.shape, inputs.attention_mask.shape)
    outputs = model(**inputs)
    print(outputs)
    last_hidden_states = outputs.last_hidden_state
    print(last_hidden_states.shape)


    import time
    count = 1
    while True:
        txt = ''.join([f"Hello, my dog is cute {i}. " for i in range(15)])
        start = time.time()
        for i in range(count):
            inputs = tokenizer(txt, return_tensors="pt")
            outputs = model(**inputs)
        end = time.time()

        last_hidden_states = outputs.last_hidden_state

        if end - start > 2:
            print("total: ", end - start)
            print("count: ", count)
            print("latency  : ", (end - start)/count)
            s_per_inf = (end - start)/count
            inf_per_s = 1/s_per_inf
            inf_per_s_32_core = 32* inf_per_s
            print("roblox inf per s", inf_per_s_32_core)
            break
        count *= 2
    print(last_hidden_states)
