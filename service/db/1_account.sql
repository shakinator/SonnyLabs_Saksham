CREATE TABLE account (
    id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    email VARCHAR(255) NOT NULL,
    username VARCHAR(100) NOT NULL,
    password VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
);

CREATE UNIQUE INDEX idx_account_username ON account(username);
CREATE UNIQUE INDEX idx_account_email ON account(email);


CREATE TABLE session (
    account_id BIGINT NOT NULL,
    token VARCHAR NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    login_info TEXT,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT fk_session_account_id
        FOREIGN KEY(account_id) REFERENCES account(id)
);

CREATE INDEX idx_session_account_id ON session(account_id, created_at);
CREATE UNIQUE INDEX idx_session_token ON session(token);
