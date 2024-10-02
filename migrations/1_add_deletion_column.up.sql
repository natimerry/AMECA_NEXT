CREATE table deleted(
    msg_id BIGINT references message(msg_id),
    author_id BIGINT references member(member_id),
    CONSTRAINT __deleted_msg_pk PRIMARY KEY (msg_id,author_id)
);