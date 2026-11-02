package mpris

import (
	"time"

	"github.com/godbus/dbus/v5"
	"github.com/godbus/dbus/v5/prop"
)

type MprisDBusObject struct {
	Properties prop.Map

	OnPause         func()
	OnPlay          func()
	OnPlayPause     func()
	OnSeek          func(time.Duration)
	OnSetPosition   func(time.Duration)
	OnTrackNext     func()
	OnTrackPrevious func()

	OnQuit  func()
	OnRaise func()
}

type LoopStatus string

const (
	LoopNone     LoopStatus = "None"
	LoopTrack    LoopStatus = "Track"
	LoopPlaylist LoopStatus = "Playlist"
)

type PlaybackStatus string

const (
	PlaybackStatusPlaying = "Playing"
	PlaybackStatusPaused  = "Paused"
	PlaybackStatusStopped = "Stopped"
)

func NewMprisDBusObject(desktopEntry string, identity string) *MprisDBusObject {
	return &MprisDBusObject{
		Properties: prop.Map{
			mediaPlayerInterface: {
				"CanQuit": {
					Value:    true,
					Writable: false,
					Emit:     prop.EmitTrue,
				},
				"CanRaise": {
					Value:    true,
					Writable: false,
					Emit:     prop.EmitTrue,
				},
				"CanSetFullscreen": {
					Value:    false,
					Writable: false,
					Emit:     prop.EmitConst,
				},
				"DesktopEntry": {
					Value:    desktopEntry,
					Writable: false,
					Emit:     prop.EmitConst,
				},
				"Fullscreen": {
					Value:    false,
					Writable: false,
					Emit:     prop.EmitTrue,
				},
				"HasTrackList": {
					Value:    false,
					Writable: false,
					Emit:     prop.EmitConst,
				},
				"Identity": {
					Value:    identity,
					Writable: false,
					Emit:     prop.EmitConst,
				},
			},
			playerInterface: {
				"CanControl": {
					Value:    true,
					Writable: false,
					Emit:     prop.EmitConst, // According to the MPRIS spec, should not be emitted
				},
				"CanGoNext": {
					Value:    true,
					Writable: false,
					Emit:     prop.EmitTrue,
				},
				"CanGoPrevious": {
					Value:    true,
					Writable: false,
					Emit:     prop.EmitTrue,
				},
				"CanPause": {
					Value:    true,
					Writable: false,
					Emit:     prop.EmitConst,
				},
				"CanPlay": {
					Value:    true,
					Writable: false,
					Emit:     prop.EmitConst,
				},
				"CanSeek": {
					Value:    true,
					Writable: false,
					Emit:     prop.EmitTrue,
				},
				"LoopStatus": {
					Value:    LoopNone,
					Writable: true,
					Emit:     prop.EmitTrue,
				},
				"MaximumRate": {
					Value:    1.0,
					Writable: false,
					Emit:     prop.EmitConst,
				},
				"Metadata": {
					Value:    map[string]any(nil),
					Writable: false,
					Emit:     prop.EmitTrue,
				},
				"MinimumRate": {
					Value:    1.0,
					Writable: false,
					Emit:     prop.EmitConst,
				},
				"PlaybackStatus": {
					Value:    PlaybackStatusStopped,
					Writable: false,
					Emit:     prop.EmitTrue,
				},
				"Position": {
					Value:    int64(0),
					Writable: false,
					Emit:     prop.EmitFalse, // According to the MPRIS spec, should not be emitted
				},
				"Rate": {
					Value:    1.0,
					Writable: false,
					Emit:     prop.EmitConst,
				},
				"Shuffle": {
					Value:    false,
					Writable: true,
					Emit:     prop.EmitTrue,
				},
				"Volume": {
					Value:    1.0,
					Writable: true,
					Emit:     prop.EmitTrue,
				},
			},
		},
	}
}

func (o *MprisDBusObject) Next() *dbus.Error {
	if onTrackNext := o.OnTrackNext; onTrackNext != nil {
		onTrackNext()
		return nil
	}

	return &dbus.ErrMsgUnknownMethod
}

func (o *MprisDBusObject) Pause() *dbus.Error {
	if onPause := o.OnPause; onPause != nil {
		onPause()
		return nil
	}

	return &dbus.ErrMsgUnknownMethod
}

func (o *MprisDBusObject) Play() *dbus.Error {
	if onPlay := o.OnPlay; onPlay != nil {
		onPlay()
		return nil
	}

	return &dbus.ErrMsgUnknownMethod
}

func (o *MprisDBusObject) PlayPause() *dbus.Error {
	if onPlayPause := o.OnPlayPause; onPlayPause != nil {
		onPlayPause()
		return nil
	}

	return &dbus.ErrMsgUnknownMethod
}

func (o *MprisDBusObject) Previous() *dbus.Error {
	if onTrackPrevious := o.OnTrackPrevious; onTrackPrevious != nil {
		onTrackPrevious()
		return nil
	}

	return &dbus.ErrMsgUnknownMethod
}

func (o *MprisDBusObject) Quit() *dbus.Error {
	if onQuit := o.OnQuit; onQuit != nil {
		onQuit()
		return nil
	}

	return &dbus.ErrMsgUnknownMethod
}

func (o *MprisDBusObject) Raise() *dbus.Error {
	if onRaise := o.OnRaise; onRaise != nil {
		onRaise()
		return nil
	}

	return &dbus.ErrMsgUnknownMethod
}

func (o *MprisDBusObject) Seek(offsetUs int64) *dbus.Error {
	if onSeek := o.OnSeek; onSeek != nil {
		onSeek(time.Duration(offsetUs) * time.Microsecond)
		return nil
	}

	return &dbus.ErrMsgUnknownMethod
}

func (o *MprisDBusObject) SetPosition(_ dbus.ObjectPath, offsetUs int64) *dbus.Error {
	if onSetPosition := o.OnSetPosition; onSetPosition != nil {
		onSetPosition(time.Duration(offsetUs) * time.Microsecond)
		return nil
	}

	return &dbus.ErrMsgUnknownMethod
}
