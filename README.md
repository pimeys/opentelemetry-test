# opentelemetry-test

Testing the Jaeger client/server with this.

## Running

1) Run Jaeger in Docker ([docs](https://www.jaegertracing.io/docs/1.22/getting-started/))
1) Start the server: `cargo run -- server`
1) Execute a request with the client: `cargo run -- client`
1) See from [Jaeger](http://localhost:16686/) the newly created span for `tracign test` service. Should have trace name `client test` with three spans.
