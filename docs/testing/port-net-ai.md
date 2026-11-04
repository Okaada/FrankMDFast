# Test port: frankmd-net (egress) and frankmd-ai

The tests compile (`cargo test -p frankmd-net -p frankmd-ai --no-run`) and will fail at runtime on `todo!()` until the code is implemented. `cargo clippy --tests` reports only `todo` warnings.

## frankmd-net

| Original | Rust | Original tests | Ported | Skipped |
|---|---|---:|---:|---|
| `test/services/egress_policy_test.rb` | `crates/frankmd-net/tests/egress_policy_test.rs` | 11 | 11 | none |
| (derived) the `images_service_test.rb` fetch tests and the `image_import_test.rb` redirect test | `crates/frankmd-net/tests/egress_client_test.rs` | n/a | 10 | n/a: `derived from:` tests of the `EgressClient::fetch` contract. The media-level ports are in frankmd-media. |
| golden `egress_policy.json` | `crates/frankmd-net/tests/golden_egress_policy.rs` | 220 cases | 220 | none |

How the fixture cases are wired:
- `blocked_ip?` (131 cases) goes through `parse_ip`, then `is_blocked_net`.
- `checked_target (resolve stubbed)` (54) calls `check_target` with a resolver that answers `resolve_returns` for any host and records which hosts were asked for.
- `checked_target (real resolve)` (20) and `resolve (real)` (15) use the system resolver.

### API changes (additive)

- New `HostResolver` trait, with `SystemResolver` and `StaticResolver` (`new`, `with`, `calls`). `GuardedResolver::new` takes an `Arc<dyn HostResolver>`.
- New functions `is_blocked`, `parse_ip` (returns `IpNet`) and `is_blocked_net`.
- `check_target` is the exact port and returns `Target`, which mirrors Ruby's `URI` accessors. `checked_target` returns `(Url, IpAddr)` and is a thin wrapper over it.
- New `NetError` variants: `InvalidAddress` and `TooManyRedirects`. `Blocked` now displays the original message unchanged.
- `Limits::EXTERNAL_IMAGE` sets 25 MiB, 30 s and 5 redirects.
- New `Fetched {url, content_type, bytes}`.
- New `EgressClient` methods: `with_resolver`, `with_dial_override(SocketAddr)` (a test seam) and `limits`. `fetch(&str)` returns `Fetched`.

### Quirks and decisions

- `parse_ip` on a CIDR returns the masked network.
- **[COMPAT]** quirk: `::/0` is *not* blocked, but `0.0.0.0/0` is.
- The recorded error message is only the prefix, `"invalid address: "`.
- **Flagged:** the original *accepts* userinfo (`http://user:pass@example.com/` passes), and the golden test keeps that.
  - `check_url` in architecture.md says "no userinfo".
  - Keep `check_target` faithful to the original, and decide separately whether `EgressClient` should be stricter.
- URLs must follow Ruby `URI.parse` rules, which is why `Target` exists instead of `url::Url`:
  - IDN and non-ASCII hosts give "Invalid URL".
  - Port 65536 is accepted.
  - The host keeps its case and its brackets.
- The `localhost` cases depend on the machine's `/etc/hosts` order. They are compared as sets, and either loopback answer passes.
- `fetch` sends `User-Agent: Mozilla/5.0 (compatible; FrankMD/1.0)` and checks every redirect hop again. A relative `Location` is joined to the current URL.

## frankmd-ai

| Original | Rust | Original tests | Ported | Skipped |
|---|---|---:|---:|---|
| `test/services/ai_service_test.rb` (`AiServiceTest` 42 + `AiServiceImageGenerationTest` 5) | `crates/frankmd-ai/tests/ai_service_test.rs` | 47 | 47 | none |
| `test/controllers/ai_controller_test.rb` | `crates/frankmd-ai/tests/ai_controller_test.rs` | 17 | 17 | none |
| (derived) the wire table in backend.md §9.1 | `crates/frankmd-ai/tests/provider_wire_test.rs` | n/a | 6 | n/a |

Mocking:
- The RubyLLM mocks became wiremock servers that speak each provider's HTTP shape.
- Base URLs are injected through `Endpoints`. Ollama uses the `OLLAMA_API_BASE` value directly.
- The original `generate_image … reference_image_path` test called the real OpenRouter API. Here a mock answers 401.
- The "does not log an API key fragment" test captures `tracing` output, using the `tracing-subscriber` dev-dependency.

### API (reshaped)

- `AiService` has: `new`, `with_endpoints`, `enabled`, `available_providers`, `current_provider`, `current_model`, `provider_info`, `fix_grammar`, `image_generation_enabled`, `image_generation_model`, `image_generation_info` and `generate_image`.
- Free items: `extract_image_from_response`, `parse_openrouter_response`, `GRAMMAR_PROMPT` (verbatim) and `DEFAULT_IMAGE_GENERATION_MODEL`.
- Each `AiError` variant displays the original's message.
- Controller layer:
  - `fix_grammar_note(service, &dyn NoteSource, path)` and `generate_image_request`.
  - Both return `AiRequestError`: `BadRequest(i18n key)`, `NotFound(key)` or `Service(AiError)`.
- `ReferenceImages` is the seam for `ImagesService.find_image`, so frankmd-ai never touches the filesystem.
- New dependency: `tracing`. New dev-dependencies: `tracing-subscriber`, `wiremock`, `tokio` and testkit.

### Decisions

- OpenAI goes through `/v1/chat/completions`. ruby_llm 2.1.1 used the Responses API instead; the user can't tell the difference.
- Anthropic gets the prompt in `system`.
- Gemini gets it in `systemInstruction`, with the key in the `x-goog-api-key` header and never as `?key=`.
- Image generation:
  - Without a reference image, the prompt is sent verbatim as plain string content.
  - With one, the content is two parts: `[text, image_url data-URI]`.
  - The image is read from `choices[0].message.images[].image_url.url`.

### What these tests need from frankmd-config

- `ResolvedConfig::new` and `EnvSnapshot::from_pairs`.
- `ai_providers_available` and `effective_ai_provider`.
- `effective_ai_model`:
  - defaults: `gpt-4o-mini`, `llama3.2:latest`, `claude-sonnet-4-20250514`, `gemini-2.0-flash` and `openai/gpt-4o-mini`,
  - `AI_MODEL` and `<PROVIDER>_MODEL` overrides,
  - an `AI_PROVIDER` override when that provider is available.
- `image_generation_key`, `get(Key::ImageGenerationModel)` and `AiProviderId`.
