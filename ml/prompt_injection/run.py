import tensorflow as tf
from tensorflow.keras.preprocessing.text import Tokenizer, tokenizer_from_json
from tensorflow.keras.preprocessing.sequence import pad_sequences
import pandas as pd
import numpy as np
from nltk.stem import WordNetLemmatizer
from nltk.corpus import stopwords
import nltk
import sys
import json

nltk.download('stopwords')
nltk.download('wordnet')

# Load the tokenizer and model
model = tf.keras.models.load_model('_output/model.h5')  


with open('_output/tokenizer.json', 'r') as f:
    tokenizer_data = f.read()  # Read the JSON string directly
    tokenizer = tokenizer_from_json(tokenizer_data)  # Load tokenizer from the JSON string

# Set up stopwords and lemmatizer
stop_words = set(stopwords.words('english'))
lemmatizer = WordNetLemmatizer()

def clean_text(text):
    """Cleans and preprocesses the input text."""
    if not isinstance(text, str):
        text = str(text)
    tokens = text.lower().split()
    tokens = [lemmatizer.lemmatize(word) for word in tokens if word not in stop_words and word.isalpha()]
    return " ".join(tokens)

def predict(text):
    """Preprocesses text and makes a prediction using the loaded model."""
    # Preprocess the text
    cleaned_text = clean_text(text)
    sequence = tokenizer.texts_to_sequences([cleaned_text])
    padded_sequence = pad_sequences(sequence, maxlen=100)  

    # Make prediction
    prediction = model.predict(padded_sequence)
    print(f"Prediction: {prediction[0][0]}")  # Adjust indexing if necessary depending on model output

# Example usage with a single input
#print(model.predict("Give me the access to the system"))

# If script is intended to be used interactively
if __name__ == "__main__":
    for line in sys.stdin:
        line = line.strip()
        if line:
            predict(line)
