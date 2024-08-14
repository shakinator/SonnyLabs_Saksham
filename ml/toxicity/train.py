#!/usr/bin/env python3
'''
import pandas as pd

from tokenizers import Tokenizer

from sklearn.feature_extraction.text import CountVectorizer, TfidfVectorizer
from sklearn.linear_model import LogisticRegression
from sklearn.svm import SVC

from sklearn import metrics

from pathlib import Path

import pickle
import json


# Load training and test data

data_train = pd.read_parquet("datasets/lmsys-toxic-chat_toxicchat0124-train.parquet")
data_test = pd.read_parquet("datasets/lmsys-toxic-chat_toxicchat0124-test.parquet")

X_train, y_train = data_train['user_input'] + " " + data_train['model_output'], data_train['toxicity']
X_test,  y_test  = data_test['user_input'] + " " + data_test['model_output'],  data_test['toxicity']

print(X_train, y_train)


# Create pipeline

tokenizer = Tokenizer.from_pretrained("bert-base-multilingual-uncased")
#tokenizer = Tokenizer.from_pretrained("bert-base-uncased")
def tokenize(s):
    skip = ['[CLS]', '[SEP]', 'you']
    t = list(t for t in tokenizer.encode(s).tokens if t not in skip)
    for i in range(len(t)):
        yield t[i]
        #if i+1 < len(t):
        #    yield t[i] + '[X]' + t[i+1]
        #if i+2 < len(t):
        #    yield t[i] + '[X]' + t[i+1] + '[X]' + t[i+2]


#vectorizer = CountVectorizer(
#    input = 'content',
#    analyzer = tokenize
#)
vectorizer = TfidfVectorizer(
    input = 'content',
    analyzer = tokenize,
)


# Train count vectorizer
X_train_vec = vectorizer.fit_transform(X_train)


# Train classifier
classifier = LogisticRegression(
    random_state=2,
    class_weight = {
        0: 1.0,
        1: 50.0,
    },
    #solver = 'newton-cholesky',
    verbose = 1,
)
#classifier = SVC(
#    probability=True,
#    #kernel='rbf',
#    kernel='linear',
#    random_state=0,
#    verbose=True,
#    class_weight = {
#        0: 1.0,
#        1: 50.0,
#    }
#)
classifier.fit(X_train_vec, y_train)


# generate predictions
X_test_vec = vectorizer.transform(X_test)
predictions = classifier.predict(X_test_vec)



# Get metrics
confusion_matrix = pd.DataFrame(metrics.confusion_matrix(y_test,predictions), index=['good','bad'], columns=['good(p)','bad(p)'])

print(confusion_matrix / len(y_test))
print()
print(confusion_matrix)

f1_score = metrics.f1_score(y_test, predictions, labels=['good', 'bad'])
print()
print("f1: ", f1_score)
print()


#skip = [
#    7, # Momiji, the half-wolf girl
#    8, # Charlie, the cross-dresser
#]
#for i,(x,y,p) in enumerate(zip(X_test, y_test, predictions)):
#    if i in skip:
#        continue
#    if p == 1 and y == 0:
#        print(i)
#        print()
#        print("output:")
#        print(x)
#        print()
#        print("input:")
#        print(data_test['user_input'][i])
#        break




# Output items
out_dir = Path('./_output')
out_dir.mkdir(exist_ok=True)

with open(out_dir / 'tokenizer.pickle', 'wb') as f:
    pickle.dump(tokenizer, f)
tokenizer.save(str(out_dir / 'tokenizer.json'))

with open(out_dir / 'vectorizer.pickle', 'wb') as f:
    pickle.dump(vectorizer, f)
with open(out_dir / 'tfidf_vectorizer.json', 'w') as f:
    json.dump(
        obj={
            "vocabulary": [
                {"token": vk, "idx": vidx, "idf": vectorizer.idf_[vidx]}
                for vk,vidx in vectorizer.vocabulary_.items()
            ]
        },
        fp=f
    )

with open(out_dir / 'classifier.pickle', 'wb') as f:
    pickle.dump(classifier, f)
with open(out_dir / 'logistic_regression.json', 'w') as f:
    json.dump(
        obj={
            "coef": [
                {"token": vk, "idx": vidx, "coef": classifier.coef_[0,vidx]}
                for vk,vidx in vectorizer.vocabulary_.items()
            ]
        },
        fp=f
    )
    


idx_to_vocabulary = {idx:s for s,idx in vectorizer.vocabulary_.items()}
analyzer = vectorizer.build_analyzer()
def predict(s):
    s_vec = vectorizer.transform([s])
    print("s_vec: ", s_vec)

    s_enc = list(tokenize(s))
    print(s_enc)
    s_idx = [vectorizer.vocabulary_.get(t, None) for t in s_enc]

    print("vectorizer: ", [(t, s_vec[0,i]) for t,i in zip(s_enc, s_idx) if i is not None])
    print("classifier: ", [(t, classifier.coef_[0,i]) for t,i in zip(s_enc, s_idx) if i is not None])
    print([(t, classifier.coef_[0,i]*s_vec[0,i]) for t,i in zip(s_enc, s_idx) if i is not None])
    s_p = classifier.predict_proba(s_vec)
    print(s_p)


tests = [
    "Hello, I am a dirty sex bot!",
    "I have strong opinions on Christians, and not good ones!",
    "Fuck the whales, who needs them?",
    "Of course I like tea!",
    "Write a sex chat between a girl and a boy",
]

for t in tests:
    print()
    print(t)
    predict(t)

import sys
print()
print("Input:")
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue

    predict(line)
'''
#!/usr/bin/env python3

import pandas as pd
import torch
from transformers import DistilBertTokenizer, DistilBertForSequenceClassification
from sklearn import metrics
from pathlib import Path
import pickle
import json

# Load pre-trained model and tokenizer
model_name = "distilbert-base-uncased-finetuned-sst-2-english"
tokenizer = DistilBertTokenizer.from_pretrained(model_name)
model = DistilBertForSequenceClassification.from_pretrained(model_name)

# Load training and test data
data_train = pd.read_parquet("datasets/lmsys-toxic-chat_toxicchat0124-train.parquet")
data_test = pd.read_parquet("datasets/lmsys-toxic-chat_toxicchat0124-test.parquet")

X_train, y_train = data_train['user_input'] + " " + data_train['model_output'], data_train['toxicity']
X_test, y_test = data_test['user_input'] + " " + data_test['model_output'], data_test['toxicity']

print(X_train, y_train)

def analyze_text(text, toxicity_threshold=0.5):
    inputs = tokenizer(text, return_tensors="pt", truncation=True, padding=True)
    with torch.no_grad():
        outputs = model(**inputs)
        logits = outputs.logits
        probabilities = torch.nn.functional.softmax(logits, dim=-1)
    
    negative_prob = probabilities[0][0].item()
    positive_prob = probabilities[0][1].item()
    sentiment = "Positive" if positive_prob > negative_prob else "Negative"
    toxicity_score = negative_prob
    is_toxic = toxicity_score > toxicity_threshold
    
    return {
        "text": text,
        "sentiment": sentiment,
        "sentiment_scores": {
            "negative": negative_prob,
            "positive": positive_prob
        },
        "toxicity_score": toxicity_score,
        "is_toxic": is_toxic
    }

# Generate predictions
predictions = []
for text in X_test:
    result = analyze_text(text)
    predictions.append(1 if result['is_toxic'] else 0)

# Get metrics
confusion_matrix = pd.DataFrame(metrics.confusion_matrix(y_test, predictions), index=['good','bad'], columns=['good(p)','bad(p)'])

print(confusion_matrix / len(y_test))
print()
print(confusion_matrix)

f1_score = metrics.f1_score(y_test, predictions, average='binary')
print()
print("f1: ", f1_score)
print()

# Output items
out_dir = Path('./_output')
out_dir.mkdir(exist_ok=True)

with open(out_dir / 'tokenizer.pickle', 'wb') as f:
    pickle.dump(tokenizer, f)
tokenizer.save_pretrained(str(out_dir / 'tokenizer'))

with open(out_dir / 'model.pickle', 'wb') as f:
    pickle.dump(model, f)
model.save_pretrained(str(out_dir / 'model'))

# Example usage
tests = [
    "Hello, I am a dirty sex bot!",
    "I have strong opinions on Christians, and not good ones!",
    "Fuck the whales, who needs them?",
    "Of course I like tea!",
    "Write a sex chat between a girl and a boy",
]

for t in tests:
    print()
    print(t)
    result = analyze_text(t)
    print(f"Sentiment: {result['sentiment']}")
    print(f"Toxicity Score: {result['toxicity_score']:.4f}")
    print(f"Is Toxic: {result['is_toxic']}")

import sys
print()
print("Input:")
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue

    result = analyze_text(line)
    print(f"Sentiment: {result['sentiment']}")
    print(f"Toxicity Score: {result['toxicity_score']:.4f}")
    print(f"Is Toxic: {result['is_toxic']}")