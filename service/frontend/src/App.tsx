import { PowerControl } from './components/power-control.tsx'
import { VolumeControl } from './components/volume-control.tsx'

function App() {
    return (
        <div className="w-full h-full flex flex-col justify-center items-center gap-5">
            <h1 className="text-4xl title">Remodian</h1>
            <PowerControl />
            <VolumeControl />
        </div>
    )
}

export default App
