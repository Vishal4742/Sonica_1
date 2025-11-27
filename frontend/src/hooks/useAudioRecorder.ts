import { useState, useRef, useCallback } from 'react';

export type AudioSource = 'microphone' | 'system';

interface UseAudioRecorderReturn {
    isRecording: boolean;
    startRecording: (source: AudioSource) => Promise<void>;
    stopRecording: () => Promise<Blob | null>;
    error: string | null;
}

export const useAudioRecorder = (): UseAudioRecorderReturn => {
    const [isRecording, setIsRecording] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const mediaRecorderRef = useRef<MediaRecorder | null>(null);
    const chunksRef = useRef<Blob[]>([]);

    const startRecording = useCallback(async (source: AudioSource) => {
        try {
            let stream: MediaStream;

            if (source === 'microphone') {
                // Capture from microphone
                stream = await navigator.mediaDevices.getUserMedia({ audio: true });
            } else {
                // Capture system audio
                // Chrome requires video:true even for audio-only capture
                const displayStream = await navigator.mediaDevices.getDisplayMedia({
                    audio: {
                        echoCancellation: false,
                        noiseSuppression: false,
                        autoGainControl: false,
                    },
                    video: true  // Required by Chrome, even for audio-only
                });

                // Extract only the audio tracks
                const audioTracks = displayStream.getAudioTracks();
                if (audioTracks.length === 0) {
                    throw new Error('No audio track found. Please select "Share audio" in the dialog.');
                }

                // Stop video tracks (we don't need them)
                displayStream.getVideoTracks().forEach(track => track.stop());

                // Create new stream with only audio
                stream = new MediaStream(audioTracks);
            }

            const mediaRecorder = new MediaRecorder(stream);
            mediaRecorderRef.current = mediaRecorder;
            chunksRef.current = [];

            mediaRecorder.ondataavailable = (e) => {
                if (e.data.size > 0) {
                    chunksRef.current.push(e.data);
                }
            };

            mediaRecorder.start();
            setIsRecording(true);
            setError(null);
        } catch (err) {
            console.error('Error accessing audio:', err);
            if (source === 'microphone') {
                setError('Could not access microphone. Please check permissions.');
            } else {
                setError('Could not capture system audio. Please select "Share audio" in the dialog.');
            }
        }
    }, []);

    const stopRecording = useCallback(async (): Promise<Blob | null> => {
        return new Promise((resolve) => {
            const mediaRecorder = mediaRecorderRef.current;
            if (!mediaRecorder || mediaRecorder.state === 'inactive') {
                resolve(null);
                return;
            }

            mediaRecorder.onstop = () => {
                const blob = new Blob(chunksRef.current, { type: 'audio/wav' });
                chunksRef.current = [];
                setIsRecording(false);

                // Stop all tracks
                mediaRecorder.stream.getTracks().forEach(track => track.stop());

                resolve(blob);
            };

            mediaRecorder.stop();
        });
    }, []);

    return { isRecording, startRecording, stopRecording, error };
};
