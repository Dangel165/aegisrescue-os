rule WannaCry_Ransomware {
    meta:
        description = "WannaCry Ransomware Detection"
        author      = "AegisRescue"
        date        = "2024-01-01"
        hash        = "24d004a104d4d54034dbcffc2a4b19a11f39008a575aa614ea04703480b1022c"
        severity    = "CRITICAL"

    strings:
        $magic1    = "WANACRY!" fullword
        $magic2    = "@WanaDecryptor@" fullword
        $note1     = "Ooops, your important files are encrypted." wide ascii
        $note2     = "bitcoin" nocase
        $exec1     = "icacls . /grant Everyone:F /T /C /Q" fullword
        $exec2     = "attrib +h +s" fullword
        $url1      = ".onion" fullword
        $delete_sh = "Delete shadow" nocase
        $vss       = "vssadmin delete shadows" nocase

    condition:
        uint16(0) == 0x5A4D and  // PE header
        (
            $magic1 or
            $magic2 or
            ($note1 and $note2) or
            ($exec1 and $exec2) or
            ($vss and $delete_sh)
        )
}

rule WannaCry_Encrypted_File {
    meta:
        description = "WannaCry encrypted file (.WNCRY)"
        severity    = "HIGH"

    strings:
        $wncry_magic = "WANACRY!"

    condition:
        $wncry_magic at 0
}
