const API_BASE = import.meta.env.VITE_API_BASE

export async function volumeUp() {
    await fetch(`${API_BASE}/volume/up`, { method: 'POST' })
}

export async function volumeDown() {
    await fetch(`${API_BASE}/volume/down`, { method: 'POST' })
}

export async function rapidUp() {
    await fetch(`${API_BASE}/volume/rapid-up`, { method: 'POST' })
}

export async function rapidDown() {
    await fetch(`${API_BASE}/volume/rapid-down`, { method: 'POST' })
}

export async function rapidStop() {
    await fetch(`${API_BASE}/volume/stop`, { method: 'POST' })
}

export async function setVolume(volume: number) {
    await fetch(`${API_BASE}/volume/set`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ volume }),
    })
}

export function openVolumeEvents(): EventSource {
    return new EventSource(`${API_BASE}/events`)
}

export async function powerOn() {
    await fetch(`${API_BASE}/power/on`, { method: 'POST' })
}

export async function powerOff() {
    await fetch(`${API_BASE}/power/off`, { method: 'POST' })
}

export async function setManual() {
    await fetch(`${API_BASE}/mode/manual`, { method: 'POST' })
}

export interface VolumeState {
    current: number
    expected: number
    desired: number
    off: boolean
    automatic: boolean
}
