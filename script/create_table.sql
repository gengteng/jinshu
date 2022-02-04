create table "user"
(
    id          text                    not null
        constraint user_pk
            primary key,
    external_id text                    not null,
    extension   json,
    create_time timestamp default now() not null
);

alter table "user"
    owner to jinshu;

create unique index user_external_id_uindex
    on "user" (external_id);

create table friend
(
    user_id     text                    not null,
    friend_id   text                    not null,
    comment     text,
    create_time timestamp default now() not null,
    constraint friend_pk
        primary key (user_id, friend_id)
);

alter table friend
    owner to jinshu;

create table block
(
    user_id     text                    not null,
    block_id    text                    not null,
    create_time timestamp default now() not null,
    constraint block_pk
        primary key (user_id, block_id)
);

alter table block
    owner to jinshu;

create table message
(
    id         text                    not null
        constraint message_pk
            primary key,
    timestamp  timestamp               not null,
    "from"     text                    not null,
    "to"       text                    not null,
    content    json                    not null,
    store_time timestamp default now() not null
);

alter table message
    owner to jinshu;

create table "group"
(
    id          text                    not null
        constraint group_pk
            primary key,
    name        text                    not null,
    create_time timestamp default now() not null
);

alter table "group"
    owner to jinshu;

create unique index group_name_uindex
    on "group" (name);

create table group_member
(
    group_id    text                    not null,
    user_id     text                    not null,
    group_name  text,
    create_time timestamp default now() not null,
    constraint group_member_pk
        primary key (group_id, user_id)
);

alter table group_member
    owner to jinshu;

-- example

create table app_user
(
    id          text                                  not null
        constraint app_user_pk
            primary key,
    username    text                                  not null,
    password    text                                  not null,
    gender      int       default 0                   not null,
    create_time timestamp default now()               not null,
    jinshu_id   text
);

alter table app_user
    owner to jinshu;

create unique index app_user_jinshu_id_uindex
    on app_user (jinshu_id);

create unique index app_user_mobile_uindex
    on app_user (username);







