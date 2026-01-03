import { defineConfig } from 'vitest/config';

export default defineConfig({
	test: {
		// Use a simpler setup without vitest-pool-workers
		// We'll handle Miniflare setup directly in our tests
		environment: 'node',
		globals: true,
	},
});
