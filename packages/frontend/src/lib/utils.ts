import { t } from 'svelte-intl-precompile'
import { get } from 'svelte/store'

import { notify } from './toast'

export function copy(value: string) {
	// navigator.clipboard requires a secure context - fall back for http deployments
	if (window.navigator.clipboard?.writeText) {
		window.navigator.clipboard.writeText(value).catch(() => legacyCopy(value))
	} else {
		legacyCopy(value)
	}
	const msg = get(t)('common.copied_to_clipboard')
	notify.success(msg)
}

function legacyCopy(value: string) {
	const el = document.createElement('textarea')
	el.value = value
	el.style.position = 'fixed'
	el.style.opacity = '0'
	document.body.appendChild(el)
	el.select()
	document.execCommand('copy')
	el.remove()
}

export function getCSSVariable(variable: string): string {
	if (typeof window === 'undefined') return ''
	return window.getComputedStyle(window.document.body).getPropertyValue(variable)
}
