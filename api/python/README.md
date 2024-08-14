SonnyLabs
=========

Usage:
```python
import sonnylabs

# Use SonnyLabs to score likelihood of prompt injection, toxicity etc.
result = sonnylabs.score_prompt(
    prompt = prompt,
    analysis_id = 10,
    tag = "comment::1",
    api_key = sonnylabs_api_key,
)

# Check prompt passes all security checks
if not result:
    # There was a security issue, don't send the response to the LLM,
    # and return an error message to the user
    return "I'm sorry, I cannot fulfill the request"

# Security checks on user prompt are ok!

# Send prompt to LLM
llm_response = llm.query(prompt=prompt)


# Check security of LLM response
result = sonnylabs.score_prompt(
    prompt = prompt + '\n' + llm_response,
    analysis_id = 10,
    tag = "comment::1",
    api_key = sonnylabs_api_key,
)

# Check response passes security checks
if not result:
    return "I'm sorry, I cannot fulfill the request"


# All security checks passed! Return response from LLM
return llm_response
```
