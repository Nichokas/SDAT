package mpris

import (
	"log/slog"
	"reflect"
	"slices"
	"time"

	"github.com/godbus/dbus/v5"
	"github.com/godbus/dbus/v5/prop"
)

type Server struct {
	properties     *prop.Properties
	dbusConnection *dbus.Conn
	dbusName       string
	object         *MprisDBusObject
}

func (s *Server) setProperty(property string, value any) {
	defer func() {
		if err := recover(); err != nil {
			log.Error("Failed to set "+property, "error", err)
		}
	}()

	s.properties.SetMust(playerInterface, property, value)
}

func (s *Server) Connect() {
	if slices.Contains(s.dbusConnection.Names(), s.dbusName) {
		log.Debug("name already owned, not re-requesting", "name", s.dbusName)
		return
	}

	reply, _ := s.dbusConnection.RequestName(s.dbusName, dbus.NameFlagDoNotQueue)
	if reply != dbus.RequestNameReplyPrimaryOwner {
		log.Error("name already taken", "name", s.dbusName)
	}
}

func (s *Server) Disconnect() {
	if !slices.Contains(s.dbusConnection.Names(), s.dbusName) {
		log.Debug("name not owned, not releasing", "name", s.dbusName)
		return
	}

	reply, _ := s.dbusConnection.ReleaseName(s.dbusName)
	if reply != dbus.ReleaseNameReplyReleased {
		log.Error("failed to release name", "name", s.dbusName)
	}
}

func (s *Server) Export() {
	s.properties, _ = prop.Export(s.dbusConnection, "/org/mpris/MediaPlayer2", s.object.Properties)
}

func (c *Server) OnPause(cb func()) {
	c.object.OnPause = cb
}

func (c *Server) OnPlay(cb func()) {
	c.object.OnPlay = cb
}

func (c *Server) OnPlayPause(cb func()) {
	c.object.OnPlayPause = cb
}

func (c *Server) OnQuit(cb func()) {
	c.object.OnQuit = cb
}

func (c *Server) OnRaise(cb func()) {
	c.object.OnRaise = cb
}

func (c *Server) OnSeek(cb func(offset time.Duration)) {
	c.object.OnSeek = cb
}

func (c *Server) OnSetPosition(cb func(offset time.Duration)) {
	c.object.OnSetPosition = cb
}

func (c *Server) OnTrackNext(cb func()) {
	c.object.OnTrackNext = cb
}

func (c *Server) OnTrackPrevious(cb func()) {
	c.object.OnTrackPrevious = cb
}

func (c *Server) OnLoopStatusChanged(cb func(loopStatus LoopStatus)) {
	c.object.Properties[playerInterface]["LoopStatus"].Callback = func(newVal *prop.Change) *dbus.Error {
		val, ok := newVal.Value.(string)
		if !ok {
			slog.Error("Unexpected format from client for LoopStatus value")
			return &dbus.ErrMsgInvalidArg
		}

		cb(LoopStatus(val))
		return nil
	}
}

func (c *Server) OnShuffleChanged(cb func(shuffle bool)) {
	c.object.Properties[playerInterface]["Shuffle"].Callback = func(newVal *prop.Change) *dbus.Error {
		val, ok := newVal.Value.(bool)
		if !ok {
			slog.Error("Unexpected format from client for Shuffle value")
			return &dbus.ErrMsgInvalidArg
		}

		cb(val)
		return nil
	}
}

func (c *Server) OnVolumeChanged(cb func(newVal float64)) {
	c.object.Properties[playerInterface]["Volume"].Callback = func(newVal *prop.Change) *dbus.Error {
		val, ok := newVal.Value.(float64)

		if !ok {
			slog.Error("Unexpected format from client for Volume value")
			return &dbus.ErrMsgInvalidArg
		}

		cb(val)
		return nil
	}
}

func (c *Server) SetLoopStatus(status LoopStatus) {
	oldVar, _ := c.properties.Get(playerInterface, "LoopStatus")
	oldVal, ok := oldVar.Value().(LoopStatus)

	if !ok {
		log.Error("Unexpected non-string value in D-Bus LoopStatus value")
		c.setProperty("LoopStatus", dbus.MakeVariant(status))
		return
	}

	if oldVal != status {
		c.setProperty("LoopStatus", dbus.MakeVariant(status))
	}
}

func (c *Server) SetPlaybackStatus(status PlaybackStatus) {
	oldVar, _ := c.properties.Get(playerInterface, "PlaybackStatus")
	oldVal, ok := oldVar.Value().(string)

	if !ok {
		log.Error("Unexpected non-string value in D-Bus PlaybackStatus value")
		c.setProperty("PlaybackStatus", dbus.MakeVariant(status))
		return
	}

	if PlaybackStatus(oldVal) != status {
		c.setProperty("PlaybackStatus", dbus.MakeVariant(status))
	}
}

func (c *Server) SetPosition(position time.Duration, seek bool) {
	oldVar, _ := c.properties.Get(playerInterface, "Position")
	oldVal, ok := oldVar.Value().(int64)

	newVal := position.Microseconds()

	if seek {
		c.dbusConnection.Emit(dbus.ObjectPath("/org/mpris/MediaPlayer2"), playerInterface+".Seeked", newVal)
	}

	if !ok {
		log.Error("Unexpected non-integer value in D-Bus Position value")
		c.setProperty("Position", dbus.MakeVariant(newVal))
		return
	}

	if oldVal != newVal {
		c.setProperty("Position", dbus.MakeVariant(newVal))
	}
}

func (c *Server) SetTrackMetadata(metadata map[string]any) {
	oldVar, _ := c.properties.Get(playerInterface, "Metadata")
	oldVal, ok := oldVar.Value().(map[string]any)

	if !ok {
		log.Error("Unexpected non-(string keyed map) value in D-Bus Metadata value")
		c.setProperty("Metadata", dbus.MakeVariant(metadata))
		return
	}

	if !reflect.DeepEqual(oldVal, metadata) {
		c.setProperty("Metadata", dbus.MakeVariant(metadata))
	}
}

func (c *Server) SetVolume(volume float64) {
	oldVar, _ := c.properties.Get(playerInterface, "Volume")
	oldVal, ok := oldVar.Value().(float64)

	if !ok {
		log.Error("Unexpected non-float value in D-Bus Volume value")
		c.setProperty("Volume", dbus.MakeVariant(volume))
		return
	}

	if oldVal != volume {
		c.setProperty("Volume", dbus.MakeVariant(volume))
	}
}
