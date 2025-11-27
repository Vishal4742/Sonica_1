import { useState } from 'react';
import { Header } from './components/Header';
import { Marquee } from './components/Marquee';
import { ListenButton } from './components/ListenButton';
import { ResultCard } from './components/ResultCard';
import { AudioSourceToggle } from './components/AudioSourceToggle';
import { useAudioRecorder, AudioSource } from './hooks/useAudioRecorder';
import { recognizeAudio, SongMatch } from './services/api';

function App() {
    const { isRecording, startRecording, stopRecording, error: recorderError } = useAudioRecorder();
    const [audioSource, setAudioSource] = useState<AudioSource>('microphone');
    const [match, setMatch] = useState<SongMatch | null>(null);
    const [isProcessing, setIsProcessing] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handleToggleListening = async () => {
        if (isRecording) {
            // Stop recording and process
            setIsProcessing(true);
            const audioBlob = await stopRecording();

            if (audioBlob) {
                try {
                    const result = await recognizeAudio(audioBlob);
                    if (result.match) {
                        setMatch(result.match);
                        setError(null);
                    } else {
                        setError('No match found. Try again.');
                        setMatch(null);
                    }
                } catch (err) {
                    setError('Connection error. Is the backend running?');
                    console.error(err);
                } finally {
                    setIsProcessing(false);
                }
            } else {
                setIsProcessing(false);
            }
        } else {
            // Start recording
            setMatch(null);
            setError(null);
            await startRecording(audioSource);
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
                        disabled={isRecording || isProcessing}
                    />
                </div>

                <ListenButton
                    isListening={isRecording || isProcessing}
                    onClick={handleToggleListening}
                />

                {/* Status Text */}
                <div className="mt-8 h-8">
                    {isProcessing && (
                        <p className="text-xl font-bold uppercase animate-pulse">Processing Audio...</p>
                    )}
                    {error && (
                        <p className="text-xl font-bold uppercase text-red-600 bg-white border border-black px-2">{error}</p>
                    )}
                    {recorderError && (
                        <p className="text-xl font-bold uppercase text-red-600 bg-white border border-black px-2">{recorderError}</p>
                    )}
                </div>

                <ResultCard match={match} onClose={() => setMatch(null)} />
            </main>

            <Marquee />
        </div>
    );
}

export default App;
