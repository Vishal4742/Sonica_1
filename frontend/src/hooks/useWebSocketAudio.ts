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
    sendChunk: (blob: Blob) => void;
    lastMatch: MatchResult | null;
    error: string | null;
}

export const useWebSocketAudio = (): UseWebSocketAudioReturn => {
    const [isConnected, setIsConnected] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [lastMatch, setLastMatch] = useState<MatchResult | null>(null);
    const socketRef = useRef<WebSocket | null>(null);

    const connect = useCallback(() => {
        // Prevent multiple connections
        if (socketRef.current && (socketRef.current.readyState === WebSocket.OPEN || socketRef.current.readyState === WebSocket.CONNECTING)) {
            console.log('WebSocket already connected or connecting');
            return;
        }

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
                        console.log('Match received:', data.match);
                        setLastMatch(data.match);
                    }
                } catch (e) {
                    console.error('Error parsing WebSocket message:', e);
                }
            };

            socket.onclose = () => {
                console.log('WebSocket Disconnected');
                setIsConnected(false);
                // Don't nullify socketRef here immediately, as it might cause race conditions with pending sends
                // or let the next connect call handle it.
            };

            socket.onerror = (event) => {
                console.error('WebSocket Error:', event);
                setError('WebSocket connection error');
            };

            socketRef.current = socket;
        } catch (err) {
            console.error('Connection failed:', err);
            setError('Failed to create WebSocket connection');
        }
    }, []);

    const disconnect = useCallback(() => {
        if (socketRef.current) {
            socketRef.current.close();
            socketRef.current = null;
            setIsConnected(false);
        }
    }, []);

    const sendChunk = useCallback((blob: Blob) => {
        if (socketRef.current && socketRef.current.readyState === WebSocket.OPEN) {
            console.log('WebSocket: Sending blob of size', blob.size);
            socketRef.current.send(blob);
        } else {
            console.warn('WebSocket: Cannot send, socket not open. State:', socketRef.current?.readyState);
        }
    }, []);

    // Cleanup on unmount
    useEffect(() => {
        return () => {
            if (socketRef.current) {
                socketRef.current.close();
            }
        };
    }, []);

    return { isConnected, connect, disconnect, sendChunk, lastMatch, error };
};
