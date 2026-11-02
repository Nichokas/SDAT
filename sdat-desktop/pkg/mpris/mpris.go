package mpris

import (
	"log/slog"

	"github.com/godbus/dbus/v5"
)

var log = slog.With("module", "MPRIS")

const (
	mediaPlayerInterface = "org.mpris.MediaPlayer2"
	playerInterface      = "org.mpris.MediaPlayer2.Player"
)

func NewMprisServer(name string, desktopEntry string, identity string) *Server {
	conn, err := dbus.ConnectSessionBus()
	if err != nil {
		log.Error("failed to connect to session bus", err)
	}

	object := NewMprisDBusObject(desktopEntry, identity)
	server := &Server{
		dbusConnection: conn,
		dbusName:       name,
		object:         object,
	}

	server.dbusConnection.Export(object, "/org/mpris/MediaPlayer2", mediaPlayerInterface)
	server.dbusConnection.Export(object, "/org/mpris/MediaPlayer2", playerInterface)
	return server
}
