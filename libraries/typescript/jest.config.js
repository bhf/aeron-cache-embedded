/** @type {import('ts-jest').JestConfigWithTsJest} */
module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  testMatch: ['**/*.test.ts'],
  clearMocks: true,
  // Integration tests wait on live WebSocket events against a backend that is far slower on CI than
  // locally; a short default makes those waits flaky. Per-test overrides still apply where set.
  testTimeout: 60000
};
