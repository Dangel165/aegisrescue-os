rule Generic_Rootkit_Indicators {
    meta:
        description = "Generic rootkit behavioral indicators"
        severity    = "CRITICAL"

    strings:
        $hide1 = "HideProcess" nocase
        $hide2 = "HideFile" nocase
        $hook1 = "SSDT hook" nocase
        $hook2 = "IDT hook" nocase
        $ioctl1 = "IoCreateDevice" fullword
        $driver1 = "DriverEntry" fullword
        $syscall1 = "ZwQuerySystemInformation" fullword
        $syscall2 = "NtQueryDirectoryFile" fullword
        $dbg1 = "IsDebuggerPresent" fullword
        $dbg2 = "CheckRemoteDebuggerPresent" fullword

    condition:
        uint16(0) == 0x5A4D and
        (
            ($hide1 or $hide2) or
            ($hook1 or $hook2) or
            ($ioctl1 and $driver1) or
            (2 of ($syscall1, $syscall2, $dbg1, $dbg2))
        )
}
