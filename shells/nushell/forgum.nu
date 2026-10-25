# >>> forgum (nushell) >>>
# Based on `forgum init nushell` output — Windows paths fixed for nu parsing
# (backslash escapes are invalid in nu string literals).
def --wrapped forgum [...rest] {
    if ($rest | is-empty) {
        let text = (^forgum fortune | complete).stdout
        ^forgum render --text $text --background --duration 0
    } else {
        ^forgum ...$rest
    }
}
def --wrapped forgum-init [...rest] {
    ^forgum init ...$rest
}
# <<< forgum <<<
# run `forgum config --tui` to customize your cow
