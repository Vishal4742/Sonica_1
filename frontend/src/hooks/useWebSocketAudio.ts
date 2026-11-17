import { useState, useCallback, useRef, useEffect } from 'react';

interface MatchResult {
    title: string;
    artist: string;
    score: number;
}

interface UseWebSocketAudioReturn {
    isConnected: boolean;
    connect: () => void;
    disconnect: () => void;
    startSession: () => void;
    sendChunk: (blob: Blob) => void;
    lastMatch: MatchResult | null;
    noMatchAt: number | null;
    error: string | null;
}

export const useWebSocketAudio = (): UseWebSocketAudioReturn => {
    const [isConnected, setIsConnected] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [lastMatch, setLastMatch] = useState<MatchResult | null>(null);
    const [noMatchAt, setNoMatchAt] = useState<number | null>(null);
    const socketRef = useRef<WebSocket | null>(null);
    const closingRef = useRef(false); // true when we close the socket on purpose

    const connect = useCallback(() => {
        // Prevent multiple connections
        if (socketRef.current && (socketRef.current.readyState === WebSocket.OPEN || socketRef.current.readyState === WebSocket.CONNECTING)) {
            return;
        }
        closingRef.current = false;

        try {
            // Determine WebSocket URL based on VITE_API_URL
            const apiUrl = import.meta.env.VITE_API_URL || 'http://localhost:8000';
            const wsUrl = apiUrl.replace(/^http/, 'ws') + '/ws';

            console.log('Connecting to WebSocket:', wsUrl);
            const socket = new WebSocket(wsUrl);

            socket.onopen = () => {
                console.log('WebSocket Connected');
                setIsConnected(true);
                setError(null);
            };

            socket.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    if (data.type === 'match') {
                        setLastMatch(data.match);
                    } else if (data.type === 'no_match') {
                        setNoMatchAt(Date.now());
                    }
                } catch (e) {
                    console.error('Error parsing WebSocket message:', e);
                }
            };

            socket.onclose = () => {
                console.log('WebSocket Disconnected');
                setIsConnected(false);
                // A free Hugging Face Space sleeps when idle and drops connections when it
                // restarts. Keep retrying: each attempt also wakes the Space up.
                if (!closingRef.current) setTimeout(connect, 3000);
            };

            socket.onerror = (event) => {
                console.error('WebSocket Error:', event); // onclose retries
            };

            socketRef.current = socket;
        } catch (err) {
            console.error('Connection failed:', err);
            setError('Failed to create WebSocket connection');
        }
    }, []);

    const disconnect = useCallback(() => {
        closingRef.current = true;
        if (socketRef.current) {
            socketRef.current.close();
            socketRef.current = null;
            setIsConnected(false);
        }
    }, []);

    // Tells the server a new recording starts, so it joins the slices that follow
    const startSession = useCallback(() => {
        if (socketRef.current?.readyState === WebSocket.OPEN) {
            socketRef.current.send('start');
        }
    }, []);

    const sendChunk = useCallback((blob: Blob) => {
        if (socketRef.current && socketRef.current.readyState === WebSocket.OPEN) {
            socketRef.current.send(blob);
        } else {
            console.warn('WebSocket: Cannot send, socket not open. State:', socketRef.current?.readyState);
        }
    }, []);

    // Cleanup on unmount
    useEffect(() => {
        return () => {
            closingRef.current = true;
            if (socketRef.current) {
                socketRef.current.close();
            }
        };
    }, []);

    return { isConnected, connect, disconnect, startSession, sendChunk, lastMatch, noMatchAt, error };
};
