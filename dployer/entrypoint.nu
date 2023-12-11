#!/usr/bin/env nu
def secret_to_pipe [secret: string, pipe: string] {
    let j = bws secret get $secret | from json
    let k = $j | get key
    let v = $j | get value
    print $k
    echo $"\n($k)=($v)" | save $pipe --append
}

def from_deploy [file, pipe: string] {
    let j = open $file --raw | decode utf-8 | from json
    let secrets = $j | get secrets
    for $e in $secrets { secret_to_pipe $e $pipe }
}

def main [] {
    # bash
    from_deploy deploy.json ti_dploy_pipe
    echo "\nTIDPLOY_READY=1" | save ti_dploy_pipe --append
}