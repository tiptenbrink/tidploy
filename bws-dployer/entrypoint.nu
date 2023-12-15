#!/usr/bin/env nu
def secret_to_pipe [secret: string] {
    # we request the secret from Bitwarden Secret Manager using its id, loading it as JSON
    let bws_res = do { bws secret get $secret } | complete
    let bws_exit = $bws_res | get exit_code

    if $bws_exit != 0 {
        print $"Bitwarden Secrets Manager CLI failed with output:\n ($bws_res | get stderr)"
        exit $bws_exit
    }

    let j = $bws_res | get stdout | from json
    # the key is identical to the environment variable key
    let k = $j | get key
    # the value is the actual secret
    let v = $j | get value
    # for each secret we append a newline and <KEY>=<VALUE> to the named pipe
    return $"\n($k)=($v)"
}

def from_deploy [file, pipe: string] {
    # open the file and load it as Nu's JSON representation
    let j = open $file --raw | decode utf-8 | from json
    # we get the value of secrets.ids, which is an array of id values
    let secrets = $j | get secrets
    # we call the secret_to_pipe function for each id in parallel and join them 
    # with new lines
    let output = $secrets | par-each { |e| secret_to_pipe $e } | str join "\n"
    # the result we send to the pipe
    echo $output | save $pipe --append
}

# main entrypoint
def main [] {
    # we call the from_deploy function with arguments tidploy.json and ti_dploy_pipe
    from_deploy secrets.json ti_dploy_pipe
    # once we're done we append a newline and TIDPLOY_READY=1 to the named pipe
    echo "\nTIDPLOY_READY=1" | save ti_dploy_pipe --append
}