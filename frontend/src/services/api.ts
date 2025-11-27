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
