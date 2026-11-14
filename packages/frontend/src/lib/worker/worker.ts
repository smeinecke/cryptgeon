import type { WorkerConract } from '$lib/worker'
import { packContent, unpackContent } from '@cryptgeon/shared'
import { expose, transfer } from 'comlink'

const contract: WorkerConract = {
	pack(input, password) {
		const content = packContent(input, password)
		return transfer(content, [content.data.buffer, content.extra.buffer, content.key.buffer])
	},
	unpack(data, key, extra) {
		return unpackContent(data, key, extra)
	},
}

expose(contract)
