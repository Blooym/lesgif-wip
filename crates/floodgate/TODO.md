# Todo

- [ ] Use proper error types instead of `anyhow`
- [ ] Implement clean error handling and messages in client/channel.
- [ ] Implement graceful shutdown for `ChannelConnectionHandle` that waits for all in-flight handlers to resolve.
- [ ] Investigate whether ensuring acks are sent before releasing a request Semaphore is better than the fire-and-forget approach.
- [ ] Do general cleanup and correctness checks.
- [ ] Write documentation, including a basic README.
