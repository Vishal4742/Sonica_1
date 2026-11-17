import { useState, useRef, useCallback } from 'react';

export type AudioSource = 'microphone' | 'system';

interface UseAudioRecorderReturn {
    isRecording: boolean;
    startContinuousRecording: (source: AudioSource, onChunk: (blob: Blob) => void, intervalMs?: number) => Promise<void>;
    stopRecording: () => void;
    error: string | null;
}

export const useAudioRecorder = (): UseAudioRecorderReturn => {
    const [isRecording, setIsRecording] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const mediaRecorderRef = useRef<MediaRecorder | null>(null);

    const startContinuousRecording = useCallback(async (source: AudioSource, onChunk: (blob: Blob) => void, intervalMs: number = 1000) => {
        try {
            let stream: MediaStream;
            if (source === 'microphone') {
                stream = await navigator.mediaDevices.getUserMedia({ audio: true });
            } else {
                // Chrome requires video:true even for audio-only capture
                const displayStream = await navigator.mediaDevices.getDisplayMedia({
                    audio: { echoCancellation: false, noiseSuppression: false, autoGainControl: false },
                    video: true
                });
                const audioTracks = displayStream.getAudioTracks();
                if (audioTracks.length === 0) throw new Error('No audio track found.');
                displayStream.getVideoTracks().forEach(track => track.stop());
                stream = new MediaStream(audioTracks);
            }

            // One continuous recording delivered in slices. Only the first slice carries
            // the container header, so the server joins the slices back together.
            const mediaRecorder = new MediaRecorder(stream);
            mediaRecorderRef.current = mediaRecorder;
            mediaRecorder.ondataavailable = (e) => {
                if (e.data.size > 0) onChunk(e.data);
            };
            mediaRecorder.onstop = () => {
                stream.getTracks().forEach(track => track.stop());
                setIsRecording(false);
            };
            mediaRecorder.start(intervalMs);
            setIsRecording(true);
            setError(null);
        } catch (err) {
            console.error('Error accessing audio:', err);
            setError(source === 'microphone'
                ? 'Could not access microphone. Please check permissions.'
                : 'Could not capture system audio. Please select "Share audio" in the dialog.');
            setIsRecording(false);
        }
    }, []);

    const stopRecording = useCallback(() => {
        const mediaRecorder = mediaRecorderRef.current;
        if (mediaRecorder && mediaRecorder.state !== 'inactive') {
            mediaRecorder.stop(); // onstop releases the microphone
        }
        setIsRecording(false);
    }, []);

    return { isRecording, startContinuousRecording, stopRecording, error };
};
