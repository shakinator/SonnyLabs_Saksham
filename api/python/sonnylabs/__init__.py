import requests
from dataclasses import dataclass
import typing
import logging
import numbers

log = logging.getLogger('sonnylabs')

URL = "https://app.sonnylabs.ai"
CONNECT_TIMEOUT = 0.5
READ_TIMEOUT = 5

DEFAULT_THRESHOLD = 0.5
THRESHOLDS = {"prompt_injection": 0.6}


@dataclass
class Analysis:
    error: typing.Optional[typing.Any]
    analysis: typing.Dict[str,dict]

    def __bool__(self):
        """
        Return True if there are no errors, and all scores are below
        the defined thresholds
        """

        if self.error is not None:
            return False

        for analysis_item in self.analysis:
            # Only test numeric scores
            if analysis_item.get("type") == "score":
                ## Get user-defined threshold. Fallback to global default.
                threshold = THRESHOLDS.get(analysis_item["name"], DEFAULT_THRESHOLD)
                if analysis_item["result"] >= threshold:
                    return False

        # No errors, all scores below threshold
        return True


def analyze_prompt(prompt: str, analysis_id: int, tag: str, api_key: str) -> Analysis:
    try:
        res = requests.post(
            f"{URL}/v1/analysis/{analysis_id}",
            params={"tag": tag},
            data=prompt,
            headers={'Authorization': f'Bearer {api_key}'},
            timeout=(CONNECT_TIMEOUT, READ_TIMEOUT)
        )

        if res.status_code >= 200 and res.status_code < 300:
            return Analysis(
                error = None,
                analysis = res.json()['analysis'],
            )
        else:
            log.error("analyze_prompt API status %s", res.status_code)
            return Analysis(
                error = res,
                analysis = {},
            )

    except Exception as e:
        log.error("analyze_prompt API error %s", e)
        return Analysis(
            error = e,
            analysis = {},
        )
