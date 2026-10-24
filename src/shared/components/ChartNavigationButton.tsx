import { Link } from "react-router-dom";
import { CandlestickChart } from "lucide-react";

interface ChartNavigationButtonProps {
  className?: string;
}

export function ChartNavigationButton({ className = "" }: ChartNavigationButtonProps): React.ReactElement {
  return (
    <Link
      to="/chart"
      aria-label="Open chart"
      title="Chart"
      className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl text-gray-400 transition hover:bg-[#1b1e27] hover:text-amber-300 ${className}`}
    >
      <CandlestickChart size={16} />
    </Link>
  );
}
