import React from "react";
import logoSvg from "../assets/vora-logo.svg";

interface VoraLogoProps {
  size?: number;
  className?: string;
}

export const VoraLogo: React.FC<VoraLogoProps> = ({ size = 36, className = "" }) => {
  return (
    <div
      style={{ width: size, height: size }}
      className={`relative flex items-center justify-center flex-shrink-0 select-none ${className}`}
    >
      <div className="w-full h-full rounded-full overflow-hidden flex items-center justify-center bg-transparent">
        <img
          src={logoSvg}
          alt="Vora Logo"
          className="w-full h-full object-contain select-none"
          draggable={false}
        />
      </div>
    </div>
  );
};
