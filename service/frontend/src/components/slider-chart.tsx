import { useRef, useCallback } from "react";
import {
    LineChart,
    Line,
    CartesianGrid,
    YAxis,
    ResponsiveContainer,
} from "recharts";

type HistoryPoint = {
    time: number;
    desired: number;
    expected: number | null;
    current: number | null;
};

type SliderChartProps = {
    value: number;
    history: HistoryPoint[];
    disabled?: boolean;
    onChange: (value: number) => void;
    onCommit: (value: number) => void;
};

export function SliderChart({
    value,
    history,
    disabled = false,
    onChange,
    onCommit,
}: SliderChartProps) {
    const trackRef = useRef<HTMLDivElement | null>(null);
    const draggingRef = useRef(false);
    const latestValueRef = useRef(value);

    const clamp = (v: number) => Math.max(0, Math.min(100, v));

    const posFromEvent = useCallback(
        (clientY: number): number => {
            const track = trackRef.current;
            if (!track) return value;
            const rect = track.getBoundingClientRect();
            const ratio = (clientY - rect.top) / rect.height;
            return clamp(Math.round((1 - ratio) * 100));
        },
        [value]
    );

    const startDrag = useCallback(
        (clientY: number) => {
            if (disabled) return;
            draggingRef.current = true;
            const val = posFromEvent(clientY);
            latestValueRef.current = val;
            onChange(val);
        },
        [disabled, posFromEvent, onChange]
    );

    const moveDrag = useCallback(
        (clientY: number) => {
            if (!draggingRef.current) return;
            const val = posFromEvent(clientY);
            latestValueRef.current = val;
            onChange(val);
        },
        [posFromEvent, onChange]
    );

    const endDrag = useCallback(() => {
        if (!draggingRef.current) return;
        draggingRef.current = false;
        onCommit(latestValueRef.current);
    }, [onCommit]);

    const handleMouseDown = (e: React.MouseEvent) => {
        e.preventDefault();
        startDrag(e.clientY);
        const onMove = (ev: MouseEvent) => moveDrag(ev.clientY);
        const onUp = () => {
            endDrag();
            window.removeEventListener("mousemove", onMove);
            window.removeEventListener("mouseup", onUp);
        };
        window.addEventListener("mousemove", onMove);
        window.addEventListener("mouseup", onUp);
    };

    const handleTouchStart = (e: React.TouchEvent) => {
        const touch = e.touches[0];
        startDrag(touch.clientY);
        const onMove = (ev: TouchEvent) => moveDrag(ev.touches[0].clientY);
        const onEnd = () => {
            endDrag();
            window.removeEventListener("touchmove", onMove);
            window.removeEventListener("touchend", onEnd);
        };
        window.addEventListener("touchmove", onMove, { passive: true });
        window.addEventListener("touchend", onEnd);
    };

    return (
        <div
            className={`flex flex-row gap-6 select-none ${
                disabled ? "opacity-50" : ""
            }`}
            style={{ height: "300px" }}
        >
            {/* Vertical Slider */}
            <div className="flex flex-col items-center">
                <label className="text-sm font-medium flex-shrink-0">
                    Desired
                </label>

                <div
                    ref={trackRef}
                    className={`relative flex-1 w-12 rounded-lg bg-gray-300 ${
                        !disabled ? "cursor-grab active:cursor-grabbing" : ""
                    }`}
                    onMouseDown={!disabled ? handleMouseDown : undefined}
                    onTouchStart={!disabled ? handleTouchStart : undefined}
                >
                    {/* Background track */}
                    <div className="absolute inset-0 bg-gray-200 rounded-lg opacity-30" />

                    {/* Fill from bottom to value */}
                    <div
                        className="absolute bottom-0 left-0 right-0 rounded-b-lg bg-blue-500 opacity-40 transition-all"
                        style={{
                            height: `${value}%`,
                        }}
                    />

                    {/* Handle circle */}
                    <div
                        className="absolute left-1/2 flex items-center justify-center z-10 transition-all"
                        style={{
                            top: `${100 - value}%`,
                            transform: "translate(-50%, -50%)",
                        }}
                    >
                        <div className="w-6 h-6 rounded-full bg-white border-2 border-blue-500 shadow-md ring-2 ring-blue-300" />
                    </div>
                </div>

                <span className="text-sm font-bold flex-shrink-0">{value}</span>
            </div>

            {/* Line Chart */}
            <div className="flex-1 min-w-[200px]">
                {history.length === 0 ? (
                    <div className="text-xs text-gray-400 text-center py-4">
                        Waiting for data...
                    </div>
                ) : (
                    <ResponsiveContainer width="100%" height="100%">
                        <LineChart
                            data={history}
                            margin={{ top: 5, right: 10, left: -20, bottom: 5 }}
                        >
                            <CartesianGrid
                                strokeOpacity={0.1}
                                vertical={false}
                            />
                            <YAxis domain={[0, 100]} hide={true} />
                            <Line
                                type="monotone"
                                dataKey="desired"
                                stroke="#3b82f6"
                                dot={false}
                                isAnimationActive={false}
                                strokeWidth={2}
                            />
                            <Line
                                type="monotone"
                                dataKey="expected"
                                stroke="#fbbf24"
                                dot={false}
                                isAnimationActive={false}
                                strokeWidth={2}
                                connectNulls={false}
                            />
                            <Line
                                type="monotone"
                                dataKey="current"
                                stroke="#4ade80"
                                dot={false}
                                isAnimationActive={false}
                                strokeWidth={2}
                                connectNulls={false}
                            />
                        </LineChart>
                    </ResponsiveContainer>
                )}
            </div>
        </div>
    );
}
