import { useRef, useCallback } from "react";

type VerticalSliderProps = {
    value: number;
    disabled?: boolean;
    onChange: (value: number) => void;
    onCommit: (value: number) => void;
};

export function VolumeSlider({
    value,
    disabled = false,
    onChange,
    onCommit,
}: VerticalSliderProps) {
    const trackRef = useRef<HTMLDivElement | null>(null);
    const draggingRef = useRef(false);
    const latestValueRef = useRef(value);

    const clamp = (v: number) => Math.max(0, Math.min(100, v));

    const posFromEvent = useCallback(
        (clientY: number): number => {
            const track = trackRef.current;
            if (!track) return value;
            const rect = track.getBoundingClientRect();
            // clientY maps: top of track (y=0) → 100, bottom (y=height) → 0
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
            className={`flex flex-col items-center select-none ${
                disabled ? "pointer-events-none opacity-50" : ""
            }`}
            style={{ height: "300px" }}
        >
            <label className="text-sm font-medium flex-shrink-0">Desired</label>

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
    );
}
