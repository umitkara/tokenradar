# Desktop notifications (Rust notifier). Calm and actionable (design-language.md §8).
#
# $limit: five_hour | weekly | scoped. $name: the model of a scoped weekly limit, e.g. "Fable".

# $what is notify-limit formatted for the same limit (Fluent terms cannot take variables).

notify-limit = { $limit ->
    [weekly] Weekly limit
    [scoped] { $name } weekly limit
   *[five_hour] 5-hour window
}

## Threshold. $pct is the threshold the limit reached.

notify-threshold-title = { $pct ->
    [100] { $what } is full
   *[other] { $what } reached { PCT($pct) }
}

## Projection. $eta is the time until full, in ms.

notify-projection-title = At this rate the { $limit ->
    [weekly] weekly limit
    [scoped] { $name } weekly limit
   *[five_hour] 5-hour window
} fills in { DUR($eta) }
notify-final-title = { $what } fills in { DUR($eta) }

## When the limit resets. $ms is a duration; $at a weekday and time, e.g. "Wed 10:59".

notify-resets-in = Resets in { DUR($ms) }.
notify-resets-at = Resets { $at }.

## Reset

notify-reset-title = { $what } has reset
notify-reset-body = Full capacity again.

## After a mute or quiet hours: several notices in one. The body lists their titles.

notify-summary-title = { $count } notifications while paused

## "Send test notification" in the settings.

notify-test-title = TokenRadar test notification
notify-test-body = Notifications work. Limit alerts will look like this.

## Periodic summaries (Step 17b). A click opens the overview.
## $approx: yes | no, whether the percentage is an estimate (shown with ≈).

digest-weekly-title = Last week: peak { $approx ->
    [yes] ≈
   *[no] {""}
}{ PCT($pct) }
digest-weekly-title-nopct = Last week's summary
# $hit of $windows 5-hour windows reached 100 %.
digest-five-hour-hits = { $hit ->
    [0] No 5-hour window hit the limit
   *[other] { $hit } of { $windows } 5-hour windows hit the limit
}
# $at: a weekday and time, e.g. "Wed 14:20".
digest-weekly-hit-at = Weekly limit hit { $at }
digest-weekly-hit = Weekly limit hit
digest-weekly-not-hit = Weekly limit not hit
digest-weekly-hit-unknown = Limit hit unknown (estimated week)
# $name: the model of a scoped weekly limit, e.g. "Fable".
digest-scoped-hit = { $name } full
digest-scoped-peak = { $name } { $approx ->
    [yes] ≈
   *[no] {""}
}{ PCT($pct) }
digest-cost = ${ NUM($usd, 2) } API-equivalent
digest-top-project = Top project: { $name }
# $parts: the comparisons below, joined with ", ".
digest-delta = vs previous week: { $parts }
# $delta: a signed number, e.g. "+12" or "−8"; for the cost a signed percentage.
digest-delta-peak = peak { $delta } pts
digest-delta-cost = cost { $delta }
# $pts: estimated points of the weekly limit used yesterday.
digest-daily-title = Yesterday: ≈{ NUM($pts, 1) } pts of the weekly limit
digest-daily-title-cost = Yesterday: { $cost }
digest-sessions = { $count ->
    [one] 1 session
   *[other] { $count } sessions
}
