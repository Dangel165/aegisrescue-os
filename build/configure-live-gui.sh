#!/usr/bin/env bash
# Live ISO GUI — startx + Openbox on tty1 (VirtualBox-safe, no LightDM)
configure_live_gui() {
    local chroot="$1"

    log "Live GUI 패키지 설치 중 (Xorg/startx, Openbox, WebKitGTK)..."

    chroot "${chroot}" env DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC \
        apt-get install -y --no-install-recommends \
        xserver-xorg-core xserver-xorg-video-vesa xserver-xorg-video-all \
        x11-xserver-utils x11-xkb-utils xauth xinit \
        openbox obconf nitrogen \
        libwebkit2gtk-4.1-0 libjavascriptcoregtk-4.1-0 \
        libgtk-3-0 libayatana-appindicator3-1 librsvg2-2 \
        libgl1 libegl1 libgbm1 libxcomposite1 libxdamage1 libxrandr2 \
        libasound2t64 libssl3 \
        fonts-dejavu-core fontconfig dbus-x11 \
        2>&1 | tail -15 || warn "일부 GUI 패키지 설치 실패"

    # VirtualBox/VM: prefer VESA driver (nomodeset + xdriver=vesa on cmdline)
    mkdir -p "${chroot}/etc/X11/xorg.conf.d"
    cat > "${chroot}/etc/X11/xorg.conf.d/99-aegis-vm.conf" <<'EOF'
Section "Device"
    Identifier "Device0"
    Driver     "vesa"
EndSection
Section "Screen"
    Identifier "Screen0"
    Device     "Device0"
EndSection
EOF

    mkdir -p "${chroot}/usr/share/applications"
    cat > "${chroot}/usr/share/applications/aegis-installer.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=AegisRescue Installer
Comment=Install AegisRescue to disk or USB
Exec=aegis-installer
Terminal=false
Categories=System;
EOF

    cat > "${chroot}/usr/share/applications/aegiscenter.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=AegisCenter
Comment=AegisRescue recovery control panel
Exec=aegiscenter
Terminal=false
Categories=System;
EOF

    chroot "${chroot}" usermod -aG video,input,render aegis 2>/dev/null || true

    # tty2 = fallback CLI while GUI runs on tty1
    chroot "${chroot}" systemctl enable getty@tty2.service 2>/dev/null || true
    chroot "${chroot}" systemctl set-default multi-user.target 2>/dev/null || true
    chroot "${chroot}" systemctl disable lightdm.service 2>/dev/null || true
    chroot "${chroot}" systemctl mask lightdm.service 2>/dev/null || true
}

install_live_gui_scripts() {
    local chroot="$1"
    local project_root="$2"

    mkdir -p "${chroot}/opt/aegis/bin"
    for script in aegis-live-autostart.sh aegis-xsession.sh; do
        cp "${project_root}/packaging/scripts/${script}" \
           "${chroot}/opt/aegis/bin/${script}"
        chmod 755 "${chroot}/opt/aegis/bin/${script}"
    done

    cp "${project_root}/packaging/scripts/aegis-start-gui.sh" \
       "${chroot}/etc/profile.d/aegis-start-gui.sh"
    chmod 755 "${chroot}/etc/profile.d/aegis-start-gui.sh"

    mkdir -p "${chroot}/home/aegis/.config/openbox" "${chroot}/etc/skel/.config/openbox"
    ln -sf /opt/aegis/bin/aegis-live-autostart.sh \
        "${chroot}/home/aegis/.config/openbox/autostart"
    ln -sf /opt/aegis/bin/aegis-live-autostart.sh \
        "${chroot}/etc/skel/.config/openbox/autostart"

    cat > "${chroot}/home/aegis/.xinitrc" <<'EOF'
#!/bin/sh
exec /opt/aegis/bin/aegis-xsession
EOF
    cp "${chroot}/home/aegis/.xinitrc" "${chroot}/etc/skel/.xinitrc"
    chmod 755 "${chroot}/home/aegis/.xinitrc" "${chroot}/etc/skel/.xinitrc"

    chroot "${chroot}" chown -R aegis:aegis /home/aegis 2>/dev/null || true
}
