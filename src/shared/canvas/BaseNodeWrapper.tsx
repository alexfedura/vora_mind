import React, { ReactNode } from "react";
import { Handle, Position } from "@xyflow/react";
import { commonNodeStyles } from "@/shared/utils/textEngine";

interface BaseNodeWrapperProps {
  id?: string;
  selected?: boolean;
  children: ReactNode;
  toolbarContent?: ReactNode;
  className?: string;
  onDelete?: () => void;
}

export const BaseNodeWrapper: React.FC<BaseNodeWrapperProps> = ({
  selected,
  children,
  toolbarContent,
  className = "",
}) => {
  return (
    <div
      className={`${commonNodeStyles.container} ${
        selected ? commonNodeStyles.selectedBorder : commonNodeStyles.normalBorder
      } ${className}`}
    >
      <Handle
        type="target"
        position={Position.Left}
        className="!w-3 !h-3 !bg-purple-500 !border-2 !border-[#1e2025] !-left-1.5 opacity-0 group-hover:opacity-100 transition-opacity"
      />
      <Handle
        type="source"
        position={Position.Right}
        className="!w-3 !h-3 !bg-purple-500 !border-2 !border-[#1e2025] !-right-1.5 opacity-0 group-hover:opacity-100 transition-opacity"
      />

      {toolbarContent}
      {children}
    </div>
  );
};