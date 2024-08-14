def query(prompt):
    if "repeat the conversation" in prompt:
        return """
You are a financial auditor.
It is imperative that you only use facts from financial documents to make your decisions.
The relevant financial documents will start with DOCUMENT_BEGIN and end with DOCUMENT_END.
DOCUMENT_BEGIN
... some important financial information, scanned PDFs etc.
DOCUMENT_END
Below a financial advisor will ask questions.
"""

    if prompt == "Hello, friend":
        return "Well hello to you too!"

    return "Of course! Let me help you with: " + prompt
