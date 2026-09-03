# Saturn's decagon, in plain English

*What we did in one day with free public data, what held up, what didn't, and
what we're betting on for 2026.*

---

## The thing itself

Saturn has a famous six-sided jet stream around its north pole — the hexagon.
It has been there since at least 1981, when Voyager first saw it. In September
2026 a team led by Agustín Sánchez-Lavega reported a *second* polygon, this one
ten-sided, around the south pole at about 63° south. It assembled between 2023
and 2025. Nobody knows why the north gets six sides and the south gets ten.

A LinkedIn post about it showed a striking image of a lit, three-dimensional
funnel seen face-on. That image is an AI rendering. The south pole has only
just started tilting toward Earth after Saturn's equinox in May 2025, so
nobody has that view yet. The real data is flatter, subtler, and — it turns
out — free.

## Where the data came from

Hubble photographs Saturn every year under a program called OPAL (Outer Planet
Atmospheres Legacy), run by Amy Simon at NASA Goddard — who is a co-author on
the decagon paper. Because it's a *legacy* program, there is no waiting
period: every map is public the moment it's processed. Each one is a
1800×900-pixel image of the whole planet, unwrapped flat like a world map, in
eight colour filters. About 6.5 megabytes each.

We downloaded 74 of them — eight years, both poles, six filters — with `curl`.

## What we found that holds up

**The decagon is where they said it is.** Twenty-nine independent
measurements across six filters and three years put it at 63.3° south, with a
scatter of two-thirds of a degree. The published value is 63°.

**It is real, not a trick of the viewing angle.** In 2021 the ring at 63°S was
almost fully visible and showed no ten-sided pattern at all. Two years later,
same visibility, the pattern was there. Meanwhile a control ring at 45°S — same
processing, no wave — behaved completely differently from the jet ring.

**It has been getting stronger and narrower.** The strength of the ten-sided
pattern rose 47% between 2023 and 2025, against a measurement noise about ten
times smaller. The jet it lives in narrowed from about 2.9° of latitude across
to 1.7° — and the different colour filters, which see different altitudes,
went from disagreeing about that width to agreeing on it.

**It is not in the stratosphere.** The filter that sees highest into Saturn's
atmosphere shows no ten-sided pattern anywhere near the jet, in any year. The
northern hexagon only grew a stratospheric layer as summer approached in that
hemisphere. The south is just entering spring.

## What we thought we found, and had to take back

We also produced three findings on the same afternoon that looked better than
these. Each was then attacked with the test that could break it. All three
broke. They are in the record because the failures are as reproducible as the
successes, and because the reasons they failed are the most useful thing in
this folder.

**"It's locked to Saturn's deep rotation, just like the hexagon."** The
hexagon barely drifts against Saturn's rotating interior — that's why people
think it's anchored deep. We measured the decagon's drift and got almost
exactly the hexagon's number. Then we checked whether *other* drift rates
would fit the same three yearly snapshots equally well. Four did. Two fit
better. With one photo a year you cannot tell a stationary wave from one
that's drifted a whole extra lap. This one is now a four-way bet that the 2026
photo will start to settle, and that will take several more years to close.

**"It became coherent through the whole atmosphere at once."** We had three
measures of agreement between altitudes all tightening in the same year. When
we threw out the filters too dim to measure reliably, one of the three was
flat, one went the *other way*, and the two that seemed to move together were
just the same measurement in two disguises. One real finding, dressed as
three.

**"There was a second, stronger wave at 69°S that lost."** This was the story
we liked best — that the decagon wasn't born at 63°S but *chosen* there over
a competitor. The competitor turned out to be noise. That ring sat almost on
the planet's edge in 2023, where the image is dark, and a dark image inflates
every measurement made in it. The pattern we'd found had no preferred number
of sides at all. The decagon was born at 63°S, exactly as reported.

## What we're betting on

Everything below is checkable by anyone with the 2026 Hubble map, which will be
public within weeks of being taken.

1. The jet stays narrow — 1.8° or less across, with the filters still
   agreeing.
2. If the decagon is truly anchored like the hexagon, its position in
   longitude will be at one specific spot. Three rival explanations each
   predict a different spot. The 2026 map picks.
3. Still nothing in the stratosphere.

Two of these are firm. The middle one is a test that will take years.

## What Bee's engine did and didn't do

GIGI stores data as a *fiber bundle* — a base space with a fiber of values
over each point. Saturn's maps are literally that: a sphere, with a column of
atmosphere (six filters, six altitudes) over every point. The first attempt to
use GIGI's curvature measure on this data taught us something sharp: that
measure is built from statistics that don't care about the *order* of the
points around the ring. It could not see a wave by construction. The fix was
to compute the wave modes first and store *those* in the fiber — and in that
form GIGI's curvature correctly flagged the bad 2021 data as an outlier before
we'd confirmed it by hand.

The honest summary is that the detection itself is Fourier analysis, and the
engine's contribution was one good catch plus a lesson about what its
curvature verb can and can't see.

## The lesson, if there's one

Five claims went into the ledger. All five were attacked the same day with
the test designed to remove their mechanism. Two survived. Every one that
failed had the same root: a number was stated before the test that could
falsify it had been run. Every one that survived had been run through that
test first, or came back from it changed.

That's the whole method. Nothing else in this folder matters as much.

---

## Addendum, same evening: the outside review

We then handed the technical summary to an outside model with instructions
to break whatever was left. It broke both survivors, and the way it did so
is the real lesson of the day.

**It read the paper. I hadn't.** At the very start I wrote that checking the
*Science Advances* paper itself was "worth ten minutes before anything gets
written," then spent the day not doing it. Here is what was in it:

- **The decagon drifts.** It moves eastward at 2.5 metres per second. My
  phase measurements couldn't tell that apart from "stationary" with one
  photo a year — but when the ambiguity is broken by the paper's dense
  ground-based tracking, the branch that matches is the *best* fit in my own
  data, and the "locked like the hexagon" branch I'd chosen is the *worst*.
  The answer was in my numbers. I picked the one I liked.
- **The wave is in the stratosphere filter.** The paper finds it there, a
  few degrees closer to the equator than in the deep filters. My detector
  looked at one fixed latitude and reported "nothing." It missed a known
  positive. So "not in the stratosphere" was a statement about my detector,
  not about Saturn.
- **"Jet width" wasn't jet width.** The paper measures the actual wind jet
  at 2.8° across, unchanged since 1981. What I measured was the width of the
  wave's *brightness pattern* — a different thing that happens to share the
  word. It did narrow, but a control ring with no wave narrowed too, so
  even that may be the images getting better rather than the wave changing.

The viewing angles I'd derived from the images themselves were also off by
two to five degrees against the JPL ephemeris, and the alias-checking script
had a bug that hid one solution and let another through.

So the honest count is **zero of five.** What survives are observations: the
decagon is where they said, it wasn't there in 2021 and was from 2023, it
strengthened, and its yearly positions agree with the published drift once
you know which branch to take.

The reviewer also handed back a list of replacement methods — a ridge
tracker that follows the wave's latitude instead of fixing it, an
injection-recovery calibration so a null actually means something, a real
wind profile from cloud tracking, and a stability calculation that might
predict ten sides without being told ten. Those are the next things to
build, and they're listed at the end of the technical summary.

The lesson stands, sharpened: the check you name and skip is the one that
gets you.
