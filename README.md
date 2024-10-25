# IMPORTANT
Run `cargo sqlx prepare` with an active database connection to make sure sqlx prepares are up to date. 

`Dockerfile` as of now doesnt create an active postgres connection for building. 
`.sqlx` files provided by the repository may be outdated. In this case file an issue