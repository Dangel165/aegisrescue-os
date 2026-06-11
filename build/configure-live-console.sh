#!/usr/bin/env bash
# Console/login setup for Live ISO — multi-user target, tty1 autologin, no DM required.
configure_live_console() {
    local chroot="$1"

    mkdir -p "${chroot}/etc/systemd/system/getty@tty1.service.d"
    cat > "${chroot}/etc/systemd/system/getty@tty1.service.d/autologin.conf" <<'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin aegis --noclear --keep-baud 115200,38400,9600 %I $TERM
Type=simple
EOF

    chroot "${chroot}" groupadd -r aegis 2>/dev/null || true
    chroot "${chroot}" useradd -m -s /bin/bash -G sudo,aegis aegis 2>/dev/null || \
        chroot "${chroot}" usermod -aG aegis aegis 2>/dev/null || true
    echo "aegis:aegis" | chroot "${chroot}" chpasswd 2>/dev/null || true

    mkdir -p "${chroot}/home/aegis"
    cat > "${chroot}/etc/profile.d/aegis-boot-hint.sh" <<'EOF'
# Boot mode hint (GUI Live vs CLI vs Forensic)
if grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null; then
    _AEGIS_BOOT="CLI"
elif grep -Eq '(^| )aegis\.mode=forensic($| )' /proc/cmdline 2>/dev/null; then
    _AEGIS_BOOT="Forensic (CLI)"
elif grep -Eq '(^| )aegis\.mode=airgap($| )' /proc/cmdline 2>/dev/null; then
    _AEGIS_BOOT="Air-Gap (CLI)"
else
    _AEGIS_BOOT="Live GUI"
fi
EOF

    cat > "${chroot}/home/aegis/.bash_profile" <<'EOF'
# GUI Live: startx runs from /etc/profile.d/aegis-start-gui.sh before this file.
if grep -Eq '(^| )aegis\.cli=1($| )' /proc/cmdline 2>/dev/null; then
    if [ -f /etc/motd-aegis ]; then cat /etc/motd-aegis; fi
    if [ -f /etc/profile.d/aegis-boot-hint.sh ]; then . /etc/profile.d/aegis-boot-hint.sh; fi
    echo ""
    echo "  Boot mode: ${_AEGIS_BOOT:-CLI}"
    echo "  Commands: aegis | aegis-installer | aegiscenter (needs: startx)"
    echo ""
elif [ -z "${DISPLAY:-}" ] && [ -f /tmp/aegis-startx.log ]; then
    echo ""
    echo "  GUI start failed:"
    echo "    /tmp/aegis-startx.log"
    echo "    /var/log/Xorg.0.log"
    echo "  Fallback: Alt+F2 for CLI  |  retry: startx -- :0 vt1"
    echo ""
fi
EOF
    cp "${chroot}/home/aegis/.bash_profile" "${chroot}/etc/skel/.bash_profile"
    chroot "${chroot}" chown aegis:aegis /home/aegis/.bash_profile 2>/dev/null || true

    cat > "${chroot}/etc/motd-aegis" <<'EOF'

  AegisRescue Beta 0.1.0 — Live Recovery OS
  Boot menu: Live (GUI+Installer) | CLI Mode | Forensic | Air-Gap
  GUI Live: X on tty1  |  Fallback CLI: Alt+F2  |  startx log: /tmp/aegis-startx.log

EOF

    chroot "${chroot}" systemctl enable getty@tty1.service 2>/dev/null || true
    chroot "${chroot}" systemctl mask plymouth-quit-wait.service 2>/dev/null || true
}

# Shared kernel cmdline (keep in sync with build/grub/grub.cfg).
# GUI Live: no nomodeset — VirtualBox VMSVGA needs vmwgfx/KMS for Plymouth + Xorg.
LIVE_KERNEL_COMMON="boot=live components live-media-path=/live console=tty1"
LIVE_KERNEL_SAFE="boot=live components live-media-path=/live nomodeset xdriver=vesa console=tty1"
LIVE_KERNEL_GUI="${LIVE_KERNEL_COMMON} splash quiet"
LIVE_KERNEL_CLI="${LIVE_KERNEL_SAFE} aegis.cli=1 systemd.unit=multi-user.target plymouth.enable=0"
LIVE_KERNEL_FORENSIC="${LIVE_KERNEL_SAFE} aegis.mode=forensic noswap aegis.cli=1 systemd.unit=multi-user.target plymouth.enable=0"
LIVE_KERNEL_AIRGAP="${LIVE_KERNEL_SAFE} aegis.mode=airgap aegis.cli=1 systemd.unit=multi-user.target plymouth.enable=0"
LIVE_KERNEL_BASE="${LIVE_KERNEL_GUI}"
