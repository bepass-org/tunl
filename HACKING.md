# Hacking on Tunl

## Prerequisites
Before you start, make sure you have the following installed on your system:

1. [Rust](https://www.rust-lang.org/tools/install) (including `cargo`)
2. [Wrangler](https://developers.cloudflare.com/workers/wrangler/install-and-update/)
3. [Node.js](https://nodejs.org/en/download/) (Wrangler requires Node.js)

## Setting Up the Development Environment

Start by getting the code:
```sh
$ git clone https://github.com/bepass-org/tunl.git && cd tunl
```

Install the dependencies:
```sh
$ rustup update
$ npm install wrangler --save-dev
```

[Create an API token](https://developers.cloudflare.com/fundamentals/api/get-started/create-token/) from the cloudflare dashboard.

Create a `.env` file based on `.env.example` and fill the values based on your tokens:
```sh
$ cp .env.example .env
$ sed -i 's/test/YOUR_CF_API_TOKEN/g' .env
```

Run the local server and start hacking:
```sh
$ make dev
```

**NOTE**: If your changes modify the configuration file, ensure you run `make schema` before submitting your patch.
