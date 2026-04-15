import { Button as HeadlessButton } from "@headlessui/react"
import type React from "react";
import { useRef, useState } from "react";

export type ButtonProps = {
    children: React.ReactNode,
    singleFunction?: () => void,
    multiFunction?: () => void,
    rapidStart?: () => void,
    rapidEnd?: () => void,
    longPressDelay?: number,
    repeatInterval?: number,
}

export function Button(props: ButtonProps) {
    const longPressDelay = props.longPressDelay ?? 400;
    const repeatInterval = props.repeatInterval ?? 250;

    const [clicking, setClicking] = useState(false);
    const timerRef = useRef<number | null>(null);
    const repeatingRef = useRef(false);
    const rapidFiringRef = useRef(false);
    const pressingRef = useRef(false); // track if press started
    const buttonRef = useRef<HTMLButtonElement | null>(null);

    const startPress = (event: React.MouseEvent | React.TouchEvent) => {
        if (!buttonRef.current) return;

        const targetElement = event.target as Node;
        if (!buttonRef.current.contains(targetElement)) return;

        pressingRef.current = true;
        repeatingRef.current = false;
        rapidFiringRef.current = false;

        timerRef.current = window.setTimeout(() => {
            if (props.rapidStart) {
                rapidFiringRef.current = true;
                click();
                props.rapidStart();
            } else if (props.multiFunction) {
                multiFunction();
                repeatingRef.current = true;
                timerRef.current = window.setTimeout(function repeat() {
                    multiFunction();
                    timerRef.current = window.setTimeout(repeat, repeatInterval);
                }, repeatInterval);
            }
        }, longPressDelay);
    };

    const endPress = () => {
        if (!pressingRef.current) return; // only fire if press started

        if (timerRef.current) {
            clearTimeout(timerRef.current);
            timerRef.current = null;
        }

        if (rapidFiringRef.current) {
            click();
            props.rapidEnd?.();
            rapidFiringRef.current = false;
        } else if (!repeatingRef.current) {
            singleFunction();
        }

        pressingRef.current = false; // reset
    };

    function singleFunction() {
        click()
        props.singleFunction?.();
    }

    function multiFunction() {
        click()
        props.multiFunction?.();
    }

    function click() {
        setClicking(true)
        setTimeout(() => setClicking(false), 20)
    }

    const clickingClassName = clicking ? 'hover:bg-gray-500' : 'hover:bg-gray-400'
    const className = 'bg-gray-300 py-2 px-4 rounded-xl border-2 cursor-pointer select-none border-gray-200 hover:border-gray-300 active:bg-gray-400 active:border-gray-300 focus:outline-blue-500 ' + clickingClassName

    console.log(className)

    return (
        <HeadlessButton className={className}
            ref={buttonRef}
            onMouseDown={startPress}
            onMouseUp={endPress}
            onMouseLeave={endPress}
            onTouchStart={startPress}
            onTouchEnd={endPress}
            onTouchCancel={endPress}>{props.children}</HeadlessButton>
    )
}
