rule Generic_Ransomware_Behavior {
    meta:
        description = "Generic ransomware behavioral indicators"
        author      = "AegisRescue"
        severity    = "HIGH"

    strings:
        // 랜섬노트 패턴
        $note1 = "your files have been encrypted" nocase
        $note2 = "your files are encrypted" nocase
        $note3 = "all your files" nocase
        $note4 = "decrypt your files" nocase
        $note5 = "pay the ransom" nocase

        // 비트코인/결제
        $btc1 = "bitcoin" nocase
        $btc2 = "BTC" fullword
        $btc3 = ".onion" fullword
        $btc4 = "cryptocurrency" nocase

        // 시스템 변조
        $sys1 = "vssadmin delete shadows" nocase
        $sys2 = "bcdedit /set" nocase
        $sys3 = "wbadmin delete catalog" nocase
        $sys4 = "wevtutil cl" nocase

        // 파일 암호화 패턴
        $enc1 = "AES" fullword
        $enc2 = "RSA" fullword
        $enc3 = "CryptEncrypt" fullword
        $enc4 = "BCryptEncrypt" fullword

    condition:
        (($note1 or $note2 or $note3 or $note4 or $note5) and
         ($btc1 or $btc2 or $btc3)) or
        ($sys1 and ($enc1 or $enc2 or $enc3 or $enc4))
}
