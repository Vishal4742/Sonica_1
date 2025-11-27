import React from 'react';
import { motion } from 'framer-motion';

export const Marquee: React.FC = () => {
    return (
        <div className="border-t-2 border-black bg-white overflow-hidden flex items-center h-[80px] relative z-20">
            <motion.div
                className="whitespace-nowrap font-bold text-2xl uppercase tracking-widest"
                animate={{ x: [0, -1000] }}
                transition={{
                    repeat: Infinity,
                    ease: "linear",
                    duration: 20,
                }}
            >
                LISTENING NOW • DETECTING AUDIO • MATCHING FINGERPRINT • SONICA ENGINE • LISTENING NOW • DETECTING AUDIO • MATCHING FINGERPRINT • SONICA ENGINE •
            </motion.div>
        </div>
    );
};
