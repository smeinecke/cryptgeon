<script lang="ts" module>
	export type DecryptedNote = {
		meta: { type: 'text' | 'file' }
		contents: any
	}

	function saveAs(file: File) {
		const url = window.URL.createObjectURL(file)
		const a = document.createElement('a')
		a.style.display = 'none'
		a.href = url
		a.download = file.name
		document.body.appendChild(a)
		a.click()
		// defer revocation - revoking synchronously can abort the download
		setTimeout(() => window.URL.revokeObjectURL(url), 0)
		a.remove()
	}
</script>

<script lang="ts">
	import prettyBytes from 'pretty-bytes'
	import { t } from 'svelte-intl-precompile'

	import Button from '$lib/ui/Button.svelte'
	import { copy } from '$lib/utils'
	import type { FileDTO } from '@cryptgeon/shared'

	interface Props {
		note: DecryptedNote
	}

	let { note }: Props = $props()

	const RE_URL = /https?:\/\/([A-Za-z0-9\-._~:\/?#\[\]@!$&'()*+,;%=])+/g
	let files: FileDTO[] = $state([])
	let previews: string[] = $state([])

async function downloadFile(file: FileDTO) {
		const f = new File([file.data.slice(0)], file.name, {
			type: file.mime,
		})
		saveAs(f)
	}

	$effect(() => {
		const list: FileDTO[] = note.meta.type === 'file' ? note.contents : []
		files = list
		const urls = list.map((f) =>
			f.mime?.startsWith('image/')
				? URL.createObjectURL(new Blob([f.data.slice(0)], { type: f.mime }))
				: ''
		)
		previews = urls
		return () => {
			for (const url of urls) if (url) URL.revokeObjectURL(url)
		}
	})
	function downloadAll() {
		for (const file of files) {
			downloadFile(file)
		}
	}
	let links = $derived(typeof note.contents === 'string' ? note.contents.match(RE_URL) : [])
</script>

<p class="error-text">{@html $t('show.warning_will_not_see_again')}</p>
<div data-testid="result">
	{#if note.meta.type === 'text'}
		<div class="note">
			{note.contents}
		</div>
		<Button onclick={() => copy(note.contents)}>{$t('common.copy_clipboard')}</Button>

		{#if links && links.length}
			<div class="links">
				{$t('show.links_found')}
				<ul>
					{#each links as link}
						<li>
							<a href={link} target="_blank" rel="noopener noreferrer">{link}</a>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	{:else}
		{#each files as file, index}
			<div class="note file">
				<button onclick={() => downloadFile(file)}>
					<b>↓ {file.name}</b>
				</button>
				<small> {file.mime} － {prettyBytes(file.size ?? file.data.length)}</small>
			</div>
			{#if previews[index]}
				<img src={previews[index]} alt={file.name} class="preview" />
			{/if}
		{/each}
		<Button onclick={downloadAll}>{$t('show.download_all')}</Button>
	{/if}
</div>

<style>
	.note {
		width: 100%;
		margin: 0;
		padding: 0;
		border: 2px solid var(--ui-bg-1);
		outline: none;
		padding: 0.5rem;
		white-space: pre;
		overflow: auto;
		margin-bottom: 0.5rem;
	}

	.note b {
		cursor: pointer;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.note.file {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.note.file small {
		padding-left: 1rem;
	}

	.links {
		margin-top: 2rem;
	}
	.links ul {
		margin: 0;
		padding: 0;
		margin-top: 0.5rem;
		padding-left: 1rem;
		list-style: square;
	}

	.links ul li {
		margin-bottom: 0.5rem;
		word-wrap: break-word;
	}

	.preview {
		display: block;
		max-width: 100%;
		max-height: 60vh;
		margin-bottom: 0.5rem;
		border-radius: 0.25rem;
		border: 2px solid var(--ui-bg-1);
	}
</style>
