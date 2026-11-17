export interface SongMatch {
    title: string;
    artist: string;
    score: number;
}

export interface RecognitionResponse {
    match: SongMatch | null;
}

export const recognizeAudio = async (audioBlob: Blob): Promise<RecognitionResponse> => {
    const formData = new FormData();
    formData.append('file', audioBlob, 'recording.wav');

    try {
        const API_URL = import.meta.env.VITE_API_URL || '/api';
        const response = await fetch(`${API_URL}/recognize`, {
            method: 'POST',
            body: formData,
        });

        if (!response.ok) {
            throw new Error(`Recognition failed: ${response.statusText}`);
        }

        const data = await response.json();
        return data;
    } catch (error) {
        console.error('Error recognizing audio:', error);
        throw error;
    }
};

export interface CleanMatch {
    title: string;
    artist: string; // "Artist not tagged" when the file has no artist
    untagged: boolean;
    pct: number; // the server's confidence, 0–100
}

// Song files often carry download-site junk in their names. Drop it, read
// "Artist - Title" when the artist tag is missing, and show " | " as " · ".
export const cleanMatch = (m: SongMatch): CleanMatch => {
    let title = m.title.replace(/^\s*video\s*-\s*/i, '');
    let artist = m.artist;
    title = title.replace(/\(MP3_.*?\)|\[.*?\]|\(.*?kbps.*?\)|www\..*?\.[a-z]+|\.mp3|\.wav/gi, '').trim();
    title = title.replace(/_/g, ' ').replace(/\s+/g, ' ').trim();
    if (artist === 'Unknown' || artist === '') {
        const sep = title.includes(' - ') ? ' - ' : title.includes(' _ ') ? ' _ ' : null;
        if (sep) {
            const parts = title.split(sep);
            artist = parts[0].trim();
            title = parts.slice(1).join(sep).trim();
        }
    }
    title = title.replace(/\s*\|\s*/g, ' · ');
    const untagged = !artist || artist === 'Unknown';
    return {
        title,
        artist: untagged ? 'Artist not tagged' : artist,
        untagged,
        pct: Math.min(100, Math.max(0, Math.round(m.score * 100))),
    };
};
