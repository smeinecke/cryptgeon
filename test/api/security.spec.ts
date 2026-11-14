import { expect, test } from '@playwright/test'
import { decode, encode } from '@msgpack/msgpack'

function createBody(meta: object, data: number[] = [1, 2, 3]) {
	return Buffer.from(encode({ meta, data: new Uint8Array(data) }))
}

async function createNote(request: any, meta: object): Promise<string> {
	const res = await request.post('/api/v3/notes/', { data: createBody(meta) })
	expect(res.status()).toBe(200)
	const body = decode(new Uint8Array(await res.body())) as any
	expect(typeof body.id).toBe('string')
	return body.id
}

test.describe('api hardening', () => {
	test('sets security headers', async ({ request }) => {
		const res = await request.get('/api/v3/status')
		const headers = res.headers()
		expect(headers['content-security-policy']).toContain("default-src 'self'")
		expect(headers['x-content-type-options']).toBe('nosniff')
		expect(headers['referrer-policy']).toBe('no-referrer')
		expect(headers['x-frame-options']).toBe('DENY')
	})

	test('rejects malformed note ids', async ({ request }) => {
		for (const id of ['other:app', 'a/b', 'x'.repeat(128), 'with space', 'a.b']) {
			const enc = encodeURIComponent(id)
			expect((await request.get(`/api/v3/notes/${enc}`)).status()).toBe(404)
			expect((await request.delete(`/api/v3/notes/${enc}`)).status()).toBe(404)
		}
	})

	test('rejects invalid msgpack bodies', async ({ request }) => {
		for (const data of [Buffer.alloc(0), Buffer.from([0xc1, 0xff]), Buffer.from('not msgpack')]) {
			const res = await request.post('/api/v3/notes/', { data })
			expect(res.status()).toBe(400)
		}
	})

	test('rejects notes without any expiry policy', async ({ request }) => {
		const res = await request.post('/api/v3/notes/', { data: createBody({}) })
		expect(res.status()).toBe(400)
	})

	test('enforces the view limit under concurrency', async ({ request }) => {
		const id = await createNote(request, { views: 1 })
		const results = await Promise.all(
			Array.from({ length: 10 }, () =>
				request.delete(`/api/v3/notes/${id}`).then((r) => r.status())
			)
		)
		expect(results.filter((s) => s === 200)).toHaveLength(1)
	})

	test('note is gone after the last view', async ({ request }) => {
		const id = await createNote(request, { views: 1 })
		expect((await request.delete(`/api/v3/notes/${id}`)).status()).toBe(200)
		expect((await request.delete(`/api/v3/notes/${id}`)).status()).toBe(404)
		expect((await request.get(`/api/v3/notes/${id}`)).status()).toBe(404)
	})

	test('preview returns views and expiration metadata', async ({ request }) => {
		const id = await createNote(request, { views: 5, expiration: 30 })
		const res = await request.get(`/api/v3/notes/${id}`)
		const body = decode(new Uint8Array(await res.body())) as any
		expect(body.meta.views).toBe(5)
		expect(body.meta.expiration).toBeGreaterThan(0)
		// Preview must not consume a view.
		expect((await request.get(`/api/v3/notes/${id}`)).status()).toBe(200)
	})
})
