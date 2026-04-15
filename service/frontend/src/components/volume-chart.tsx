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

type VolumeChartProps = {
    history: HistoryPoint[];
    disabled?: boolean;
};

export function VolumeChart({ history, disabled = false }: VolumeChartProps) {
    if (history.length === 0) {
        return (
            <div
                className={`flex-1 ${disabled ? "opacity-50" : ""}`}
                style={{ minWidth: "200px" }}
            >
                <div className="text-xs text-gray-400 text-center py-4">
                    Waiting for data...
                </div>
            </div>
        );
    }

    return (
        <div
            className={`flex-1 ${disabled ? "opacity-50" : ""}`}
            style={{ minWidth: "200px" }}
        >
            <ResponsiveContainer width="100%" height={260}>
                <LineChart
                    data={history}
                    margin={{ top: 5, right: 10, left: -20, bottom: 5 }}
                >
                    <CartesianGrid strokeOpacity={0.1} vertical={false} />
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
        </div>
    );
}
