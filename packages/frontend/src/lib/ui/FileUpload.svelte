<script lang="ts">
	import { t } from 'svelte-intl-precompile'

	import Button from '$lib/ui/Button.svelte'
	import MaxSize from '$lib/ui/MaxSize.svelte'
	import type { FileDTO } from 'cryptgeon/shared'

	interface Props {
		label?: string
		files?: FileDTO[]
		[key: string]: any
	}

	let { label = '', files = $bindable([]), ...rest }: Props = $props()
	let isDragging = $state(false)
	let inputRef: HTMLInputElement | null = $state(null)

	async function fileToDTO(file: File): Promise<FileDTO> {
		return {
			name: file.name,
			size: file.size,
			type: file.type,
			contents: new Uint8Array(await file.arrayBuffer()),
		}
	}

	async function onInput(e: Event) {
		const input = e.target as HTMLInputElement
		if (input?.files?.length) {
			const toAdd = await Promise.all(Array.from(input.files).map(fileToDTO))
			files = [...files, ...toAdd]
		}
	}

	async function onDrop(e: DragEvent) {
		e.preventDefault()
		e.stopPropagation()
		// https://developer.mozilla.org/en-US/docs/Web/API/DragEvent/dataTransfer
		// "never null when dispatched by the browser"
		isDragging = false
		if (e.dataTransfer!.items.length !== 0) {
			const toAdd = await Promise.all(Array.from(e.dataTransfer!.files).map(fileToDTO))
			files = [...files, ...toAdd]
		}
	}

	function onDragEnter(e: DragEvent) {
		e.preventDefault()
		e.stopPropagation()
		isDragging = true
	}

	function onDragLeave(e: DragEvent) {
		e.preventDefault()
		e.stopPropagation()
		isDragging = false
	}

	function onDragOver(e: DragEvent) {
		e.preventDefault()
		e.stopPropagation()
	}

	function onKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault()
			inputRef?.click()
		}
	}

	function clear(e: Event) {
		e.preventDefault()
		files = []
	}
</script>

<svelte:window
	// cancels default browser behavior of downloading dragged file
	ondrop={(e: DragEvent) => {
		if ([...e.dataTransfer!.items].some((item) => item.kind === 'file')) {
			e.preventDefault()
			e.stopPropagation()
		}
	}}
	ondragover={(e: DragEvent) => {
		e.preventDefault()
		e.stopPropagation()
	}}
/>

<label>
	<small>
		{label}
	</small>
	<input bind:this={inputRef} {...rest} type="file" onchange={onInput} multiple />
	<div
		role="button"
		tabindex="0"
		class="box"
		class:file-drag={isDragging}
		ondrop={onDrop}
		ondragenter={onDragEnter}
		ondragleave={onDragLeave}
		ondragover={onDragOver}
		onkeydown={onKeyDown}
	>
		{#if files.length}
			<div>
				<b>{$t('file_upload.selected_files')}</b>
				{#each files as file}
					<div class="file">
						{file.name}
					</div>
				{/each}
				<div class="spacer"></div>
				<Button onclick={clear}>{$t('file_upload.clear')}</Button>
			</div>
		{:else}
			<div>
				<b>{$t('file_upload.no_files_selected')}</b>
				<br />
				<small>
					{$t('common.max')}: <MaxSize />
				</small>
			</div>
		{/if}
	</div>
</label>

<style>
	input {
		display: none;
	}

	.box {
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
		cursor: pointer;
	}

	.box.file-drag {
		border-color: var(--ui-clr-primary);
	}

	/* Prevent child elements from triggering dragleave during drag */
	.box.file-drag :global(*) {
		pointer-events: none;
	}

	.spacer {
		margin-top: 1rem;
	}
</style>
