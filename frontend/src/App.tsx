import { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { Marquee } from './components/Marquee';
import { ListenButton } from './components/ListenButton';
import { ResultCard } from './components/ResultCard';
import { AudioSourceToggle } from './components/AudioSourceToggle';
import { useAudioRecorder, AudioSource } from './hooks/useAudioRecorder';
import { useWebSocketAudio } from './hooks/useWebSocketAudio';
import { SongMatch } from './services/api';

function App() {
    const { isRecording, startContinuousRecording, stopRecording, error: recorderError } = useAudioRecorder();
    const { isConnected, connect, disconnect, startSession, sendChunk, lastMatch, noMatchAt, error: wsError } = useWebSocketAudio();

    const [audioSource, setAudioSource] = useState<AudioSource>('microphone');
    const [match, setMatch] = useState<SongMatch | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [status, setStatus] = useState<string>('');

    // 1. Persistent Connection: Connect on mount
    useEffect(() => {
        connect();
        return () => {
            disconnect();
        };
    }, [connect, disconnect]);

    // Handle matching
    useEffect(() => {
        if (lastMatch) {
            setMatch(lastMatch);
            setStatus('Match found!');
            stopRecording();
            // Do NOT disconnect here, keep the socket open
        }
    }, [lastMatch, stopRecording]);

    // The server gave up after ~12 s of audio without a match
    useEffect(() => {
        if (noMatchAt) {
            stopRecording();
            setStatus('');
            setError('No match found. Is this song in our catalog?');
        }
    }, [noMatchAt, stopRecording]);

    // Handle errors
    useEffect(() => {
        if (recorderError) setError(recorderError);
        if (wsError) setError(wsError);
    }, [recorderError, wsError]);

    const handleToggleListening = async () => {
        if (isRecording) {
            stopRecording();
            setStatus('');
            return;
        }

        if (!isConnected) {
            connect(); // the status line shows "Connecting to server..." meanwhile
            return;
        }

        setError(null);
        setMatch(null);
        setStatus('Listening...');

        try {
            // One continuous recording sent in 1 s slices; the server matches the growing
            // recording every second and replies with a match or, after ~12 s, no_match
            startSession();
            await startContinuousRecording(audioSource, sendChunk, 1000);

        } catch (err) {
            console.error(err);
            setError('Failed to start listening');
            setStatus('');
        }
    };

    return (
        <div className="min-h-screen bg-background text-black font-sans flex flex-col overflow-hidden relative">
            <Header />

            {/* Background Text */}
            <div className="absolute top-[60px] left-0 right-0 bottom-[80px] flex items-center justify-center pointer-events-none overflow-hidden z-0 opacity-5">
                <h1 className="text-[20vw] font-bold leading-none text-center whitespace-nowrap">
                    LISTEN<br />RECOGNIZE
                </h1>
            </div>

            {/* Decorative Tags */}
            <div className="absolute top-[100px] right-[20px] bg-white border border-black px-2 py-1 text-xs font-bold uppercase rotate-[5deg] z-10 hidden md:block">
                {audioSource === 'microphone' ? 'Microphone Active' : 'System Audio Active'}
            </div>
            <div className="absolute bottom-[120px] left-[20px] bg-accent border border-black px-2 py-1 text-xs font-bold uppercase rotate-[-5deg] z-10 hidden md:block">
                Ready to Scan
            </div>

            {/* Main Stage */}
            <main className="flex-1 flex flex-col justify-center items-center relative z-10">
                {/* Audio Source Toggle */}
                <div className="mb-8">
                    <AudioSourceToggle
                        audioSource={audioSource}
                        onToggle={setAudioSource}
                        disabled={isRecording}
                    />
                </div>

                <ListenButton
                    isListening={isRecording}
                    onClick={handleToggleListening}
                />

                {/* Status Text */}
                <div className="mt-8 h-8">
                    {!isConnected && !status && !error && (
                        <p className="text-xl font-bold uppercase animate-pulse">Connecting to server...</p>
                    )}
                    {status && (
                        <p className="text-xl font-bold uppercase animate-pulse">{status}</p>
                    )}
                    {error && (
                        <p className="text-xl font-bold uppercase text-red-600 bg-white border border-black px-2">{error}</p>
                    )}
                </div>

                <ResultCard match={match} onClose={() => setMatch(null)} />
            </main>

            <Marquee />
        </div>
    );
}

export default App;
