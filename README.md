# AI Buddy

Rust implementation of an AI using the OpenAI (Should be ClosedAI) API.

## TODOs

**FOCUS:** The main thing now is being able to use Ollama.io, instead of OpenAI, since it's a free alternative. And data won't be exposed to a third party.

## Require

- [Rust Stable](https://rustup.rs)

## How to run

### Starting dev environment

- There are 2 ways that this can be achieved:

  - Using `cargo build` and `cargo run` in sequence
  - **(Recomended)** using `cargo watch` to enable live reload of the APP and on development testing. Run the following: `cargo watch -q -c -x "run -q" for just the live-reload.

### Using OpenAI Models

**IMPORTANT:** You MUST have OpenAI credits to use it's API, otherwise you'll get an exceed limit error.

- To use the OpenAI models, you need to have an API key. You can get one by signing up at [OpenAI API Token Keys](https://platform.openai.com/api-keys).
- After getting the API key, you need to create a `.env` file in the root of the project and add the following line to it: `OPENAI_API_KEY=<your-api-key>`.
- Finally, get the assistant model in the OpenAI website and add it to the main.rs
