import { powerOff, powerOn } from "../api";
import { Button } from "./ui/button";

export function PowerControl() {
    return (
        <div className="flex flex-row gap-3">
            <Button singleFunction={powerOn}>On</Button>
            <Button singleFunction={powerOff}>Off</Button>
        </div>
    )
}
