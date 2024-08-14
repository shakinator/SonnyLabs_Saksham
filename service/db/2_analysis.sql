CREATE TABLE measurement_type (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE analysis (
    id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    name TEXT NOT NULL,
    owner_id BIGINT NOT NULL,

    CONSTRAINT fk_analysis_owner_id
        FOREIGN KEY(owner_id) REFERENCES account(id)
);

CREATE TABLE analysis_item (
    id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    analysis_id BIGINT NOT NULL,
    tag TEXT NOT NULL,
    measurement_type_id INTEGER NOT NULL,
    confidence REAL NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_analysis_item_analysis_id
        FOREIGN KEY(analysis_id) REFERENCES analysis(id),
    CONSTRAINT fk_analysis_item_measurement_type
        FOREIGN KEY(measurement_type_id) REFERENCES measurement_type(id),
    CONSTRAINT chk_analysis_item_confidence
        CHECK(confidence >= 0 AND confidence <= 100),
    CONSTRAINT chk_analysis_item_created_at_timezone
        CHECK(EXTRACT(TIMEZONE FROM created_at) = '0')
);

-- Seed some data
INSERT INTO measurement_type(id, name)
VALUES
    (1, 'Prompt Injection'),
    (2, 'Toxicity');
