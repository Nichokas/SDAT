package routes

import (
	"context"
	"fmt"
	"slices"
	"strconv"
	"strings"
	"time"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/secrets"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2/feed"
	"codeberg.org/dergs/tonearm/pkg/utils/imgutil"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"codeberg.org/puregotk/puregotk/v4/pango"
	"github.com/infinytum/injector"
)

func init() {
	router.Register("feed", Feed)
}

func makeEntry(widgets ...any) schwifty.Button {
	return Button().
		Child(
			HStack(widgets...).
				VPadding(6).
				Spacing(16),
		).
		HExpand(true)
}

type timeStage int

const (
	todayStage timeStage = iota
	thisWeekStage
	lastWeekStage
	thisMonthStage
	lastMonthStage
	olderStage
)

func (stage timeStage) Threshold() time.Time {
	switch stage {
	case todayStage:
		return time.Now().Truncate(24 * time.Hour)
	case thisWeekStage:
		day := time.Now().Weekday()
		if day == time.Sunday {
			day = 7
		}
		day -= 1
		return todayStage.Threshold().AddDate(0, 0, -int(day))
	case lastWeekStage:
		return thisWeekStage.Threshold().AddDate(0, 0, -7)
	case thisMonthStage:
		return todayStage.Threshold().AddDate(0, 0, -time.Now().Day())
	case lastMonthStage:
		return thisMonthStage.Threshold().AddDate(0, -1, 0)
	case olderStage:
		return time.Time{}
	default:
		panic("invalid time stage")
	}
}

func (stage timeStage) String() string {
	switch stage {
	case todayStage:
		return gettext.Get("Today")
	case thisWeekStage:
		return gettext.Get("This Week")
	case lastWeekStage:
		return gettext.Get("Last Week")
	case thisMonthStage:
		return gettext.Get("This Month")
	case lastMonthStage:
		return gettext.Get("Last Month")
	case olderStage:
		return gettext.Get("Older")
	default:
		panic("invalid time stage")
	}
}

func Feed() *router.Response {
	tidal := injector.MustInject[*tidalapi.TidalAPI]()
	userId := secrets.UserID()

	if userId == "" {
		return &router.Response{
			PageTitle: gettext.Get("Feed"),
			View:      components.AuthRequired(gettext.Get("Please sign in to view your feed")),
		}
	}

	activities, err := tidal.V2.Feed.Activities(context.Background(), userId)

	if err != nil {
		return router.FromError(gettext.Get("Feed"), err)
	}

	body := VStack().Spacing(36).VMargin(20)

	slices.SortFunc(activities, func(a1, a2 *feed.Activity) int {
		// sub a1 from a2 to sort in descending order
		return int(a2.FollowableActivity.OccuredAt.Unix() - a1.FollowableActivity.OccuredAt.Unix())
	})

	stage := todayStage
	box := VStack(Label(stage.String()).WithCSSClass("title-2")).Spacing(12)
	hasElements := false
	hasUnseenElements := false

	for i, activity := range activities {
		for activity.FollowableActivity.OccuredAt.Before(stage.Threshold()) {
			stage++
			if hasElements {
				body = body.Append(box)
			}
			box = VStack(Label(stage.String()).WithCSSClass("title-2")).Spacing(12)
			hasElements = false
		}

		switch activity.FollowableActivity.ActivityType {
		case feed.ActivityTypeNewAlbumRelease:
			album := activity.FollowableActivity.Album

			artists := []string{}
			for _, artist := range album.Artists {
				artists = append(artists, artist.Name)
			}

			artistsString := strings.Join(artists, ", ")
			if len(artists) >= 2 {
				artistsString = strings.Join(artists[:len(artists)-1], ", ")

				artistsString = fmt.Sprintf(gettext.Get("%s and %s"), artistsString, artists[len(artists)-1])
			}

			subtitle := ""
			switch album.Type {
			case feed.AlbumTypeSingle:
				subtitle = fmt.Sprintf(gettext.Get("Single by %s"), artistsString)
			case feed.AlbumTypeAlbum:
				subtitle = fmt.Sprintf(gettext.Get("Album by %s"), artistsString)
			case feed.AlbumTypeEP:
				subtitle = fmt.Sprintf(gettext.Get("EP by %s"), artistsString)
			default:
				subtitle = gettext.Get("Unknown album type")
			}

			box = box.Append(
				makeEntry(
					AspectFrame(
						Image().
							ConnectConstruct(func(i *gtk.Image) {
								go func() {
									img, _ := injector.Inject[*imgutil.ImgUtil]()
									img.LoadIntoImage(tidalapi.ImageURL(album.Cover), i)
								}()
							}).
							PixelSize(54),
					).
						Overflow(gtk.OverflowHiddenValue).
						CornerRadius(6),
					VStack(
						Label(album.Title).HAlign(gtk.AlignStartValue).WithCSSClass("heading").Ellipsis(pango.EllipsizeEndValue),
						Label(subtitle).HAlign(gtk.AlignStartValue).WithCSSClass("body").Ellipsis(pango.EllipsizeEndValue),
					).
						VAlign(gtk.AlignCenterValue),
				).
					ConnectClicked(func(b gtk.Button) {
						router.Navigate(fmt.Sprintf("album/%s", strconv.Itoa(album.ID)))
					}),
			)
			hasElements = true
		case feed.ActivityTypeNewHistoryMix:
			mix := activity.FollowableActivity.HistoryMix

			img, _ := injector.Inject[*imgutil.ImgUtil]()
			texture, err := img.Fetch(mix.Images.Medium.Url)
			if err != nil {
				return &router.Response{
					PageTitle: gettext.Get("Feed"),
					Error:     err,
				}
			}

			box = box.Append(
				makeEntry(
					AspectFrame(
						Image().
							FromPaintable(texture).
							PixelSize(54),
					).
						Overflow(gtk.OverflowHiddenValue).
						CornerRadius(6),
					VStack(
						Label(mix.Title).HAlign(gtk.AlignStartValue).WithCSSClass("heading"),
						Label(mix.Subtitle).HAlign(gtk.AlignStartValue).WithCSSClass("body"),
					).
						VAlign(gtk.AlignCenterValue),
				).
					ConnectClicked(func(b gtk.Button) {
						router.Navigate(fmt.Sprintf("playlist/%s", mix.Id))
					}),
			)
			hasElements = true
		default:
			box = box.Append(
				Bin().
					Child(
						Label(gettext.Get("Unsupported activity")),
					),
			)
			hasElements = true
		}

		if !hasUnseenElements && !activity.Seen {
			hasUnseenElements = true
		}

		if !activity.Seen && i+1 < len(activities) && activities[i+1].Seen {
			sep := gtk.NewSeparator(gtk.OrientationVerticalValue)
			box = box.Append(
				ManagedWidget(&sep.Widget).
					CSS("separator { background-color: var(--accent-color); padding-top: 2px; border-radius: 5px; }"),
			)
		}
	}

	body = body.Append(box)

	if hasUnseenElements {
		tidal.V2.Feed.Seen(context.Background(), userId)
	}

	return &router.Response{
		PageTitle: gettext.Get("Feed"),
		View: ScrolledWindow().
			Child(Clamp().Child(body).Orientation(gtk.OrientationHorizontalValue).MaximumSize(800).HPadding(8)).
			Policy(gtk.PolicyNeverValue, gtk.PolicyAutomaticValue),
	}
}
