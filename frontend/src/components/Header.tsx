import React from 'react';

export const Header: React.FC = () => {
    return (
        <div className="w-full border-b-2 border-black bg-white flex justify-between items-center px-5 h-[60px] relative z-20">
            <div className="font-bold text-2xl uppercase bg-black text-white px-2 py-1 tracking-tighter">
                Sonica
            </div>
            <div className="font-bold text-lg">V1.0</div>
        </div>
    );
};
