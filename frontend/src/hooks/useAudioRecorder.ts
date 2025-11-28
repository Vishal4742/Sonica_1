import { useState, useRef, useCallback } from 'react';

export type AudioSource = 'microphone' | 'system';

interface UseAudioRecorderReturn {
    isRecording: boolean;
    startRecording: (source: AudioSource) => Promise<void>;
    startContinuousRecording: (source: AudioSource, onChunk: (blob: Blob) => void, intervalMs?: number) => Promise<void>;
    stopRecording: () => Promise<Blob | null>;
    error: string | null;
}

export const useAudioRecorder = (): UseAudioRecorderReturn => {
    const [isRecording, setIsRecording] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const mediaRecorderRef = useRef<MediaRecorder | null>(null);
    const chunksRef = useRef<Blob[]>([]);
    const shouldLoopRef = useRef(false);

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

    const startContinuousRecording = useCallback(async (source: AudioSource, onChunk: (blob: Blob) => void, intervalMs: number = 3000) => {
        try {
            let stream: MediaStream;
            if (source === 'microphone') {
                stream = await navigator.mediaDevices.getUserMedia({ audio: true });
            } else {
                const displayStream = await navigator.mediaDevices.getDisplayMedia({
                    audio: { echoCancellation: false, noiseSuppression: false, autoGainControl: false },
                    video: true
                });
                const audioTracks = displayStream.getAudioTracks();
                if (audioTracks.length === 0) throw new Error('No audio track found.');
                displayStream.getVideoTracks().forEach(track => track.stop());
                stream = new MediaStream(audioTracks);
            }

            const mediaRecorder = new MediaRecorder(stream);
            mediaRecorderRef.current = mediaRecorder;
            chunksRef.current = [];
            shouldLoopRef.current = true;

            const startLoop = () => {
                if (!shouldLoopRef.current) return;

                if (mediaRecorder.state === 'inactive') {
                    console.log('Recorder: Starting new segment');
                    mediaRecorder.start();
                }

                setTimeout(() => {
                    if (shouldLoopRef.current && mediaRecorder.state === 'recording') {
                        console.log('Recorder: Stopping segment to flush');
                        mediaRecorder.stop();
                    }
                }, intervalMs);
            };

            mediaRecorder.onstop = () => {
                console.log('Recorder: onstop fired. Chunks:', chunksRef.current.length);
                const blob = new Blob(chunksRef.current, { type: 'audio/webm' });
                chunksRef.current = [];

                if (blob.size > 0) {
                    console.log('Recorder: Emitting chunk of size', blob.size);
                    onChunk(blob);
                } else {
                    console.warn('Recorder: Blob size is 0');
                }

                if (shouldLoopRef.current) {
                    startLoop();
                } else {
                    // Stop all tracks if we are done
                    mediaRecorder.stream.getTracks().forEach(track => track.stop());
                }
            };

            mediaRecorder.ondataavailable = (e) => {
                console.log('Recorder: Data available, size', e.data.size);
                if (e.data.size > 0) {
                    chunksRef.current.push(e.data);
                }
            };

            startLoop();
            setIsRecording(true);
            setError(null);

        } catch (err) {
            console.error('Error accessing audio:', err);
            setError('Could not access audio source.');
            setIsRecording(false);
        }
    }, []);

    const stopRecording = useCallback(async (): Promise<Blob | null> => {
        shouldLoopRef.current = false; // Stop the loop

        return new Promise((resolve) => {
            const mediaRecorder = mediaRecorderRef.current;
            if (!mediaRecorder || mediaRecorder.state === 'inactive') {
                setIsRecording(false);
                resolve(null);
                return;
            }

            // Let's just stop it. The existing onstop will see shouldLoopRef=false and cleanup.
            mediaRecorder.stop();
            setIsRecording(false);

            // We resolve null because in continuous mode we handle data via callback
            resolve(null);
        });
    }, []);

    return { isRecording, startRecording, startContinuousRecording, stopRecording, error };
};
