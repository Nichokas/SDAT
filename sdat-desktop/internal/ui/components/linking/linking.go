package linking

import (
	"bytes"
	"context"
	"log/slog"
	"strings"
	"time"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/resources"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/glib"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/yeqown/go-qrcode/v2"
	"github.com/yeqown/go-qrcode/writer/standard"
	"github.com/yeqown/go-qrcode/writer/standard/shapes"
)

var QRCode = Image().
	PixelSize(186).
	FromPaintable(resources.MissingAlbum())

var Code = Label("D  E  R  G  S").WithCSSClass("title-1")

var Helper = Label(gettext.Get("You can also open the linking page using the button below"))

type QRBuffer struct {
	bytes.Buffer
}

func (q *QRBuffer) Close() error {
	return nil
}

func NewLinking(window *gtk.Window, code string, link string, cancel context.CancelFunc) schwifty.AlertDialog {
	if !strings.Contains(link, "://") {
		link = "https://" + link
	}

	encodedUrl, err := qrcode.New(link)
	if err != nil {
		slog.Error("could not generate QR code to sign in")
	}

	var buf QRBuffer
	shape := shapes.Assemble(shapes.RoundedFinder(), shapes.LiquidBlock())
	writer := standard.NewWithWriter(&buf, standard.WithCustomShape(shape))

	if err := encodedUrl.Save(writer); err != nil {
		slog.Error("could not write QR code to sign in")
	}

	gBytes := glib.NewBytes(buf.Bytes(), uint(buf.Len()))
	texture, err := gdk.NewTextureFromBytes(gBytes)
	if err != nil {
		slog.Error("could not create texture from bytes")
	}

	return AlertDialog(gettext.Get("Sign In"), gettext.Get("Scan this QR code to sign into your TIDAL account")).
		WithCSSClass("no-response").
		CanClose(false).
		ConnectCloseAttempt(func(d adw.Dialog) {
			cancel()
		}).
		ExtraChild(
			VStack(
				AspectFrame(
					QRCode.FromPaintable(texture),
				).Background("alpha(var(--view-fg-color), 0.1)").HAlign(gtk.AlignCenterValue).CornerRadius(10).
					Overflow(gtk.OverflowHiddenValue),
				Code.Text(strings.Join(strings.Split(code, ""), "  ")),
				Helper,
				VStack(
					Button().
						Label(gettext.Get("Open TIDAL Page")).
						WithCSSClass("suggested-action").
						HPadding(20).VPadding(10).
						ConnectClicked(func(b gtk.Button) {
							gtk.ShowUri(window, link, uint32(time.Now().Unix()))
						}),
					Button().
						Label(gettext.Get("Cancel Login")).
						HPadding(20).VPadding(10).
						ConnectClicked(func(b gtk.Button) {
							cancel()
						}),
				).Spacing(10),
			).Spacing(20),
		)
}
