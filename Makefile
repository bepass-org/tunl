include .env

.PHONY: help
help: ## display help section
	@ cat $(MAKEFILE_LIST) | grep -e "^[a-zA-Z_\-]*: *.*## *" | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

.PHONY: schema
schema: ## generate json schema based on config.rs
	@ cargo run --bin schema_generator > config.schema.json

.PHONY: deploy
deploy: ## deploy to cf workers
	@ npx wrangler deploy

.PHONY: dev
dev: ## run the project locally
	@ npx wrangler dev --local-protocol https --port 8787
