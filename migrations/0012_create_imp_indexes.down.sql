-- Add down migration script here
DROP INDEX if exists idx_prohibited_words_by_guild;
DROP INDEX if exists idx_roles_guild_id;
DROP INDEX if exists idx_roles_name;
DROP index if exists idx_roles_guild;