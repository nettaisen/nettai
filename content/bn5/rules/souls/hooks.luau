--!strict
-- What the soul change (./change) and the revert (./revert) share: a form's start and end hooks (0x0800F024 and
-- 0x0800F088, by the soul's number, BN6's `sub_8011268` and `sub_8011384`: what the form wears, its identity's
-- parts: GyroSoul's propeller, NumberSoul's layer), and the status reset that settles MegaMan in his new form.

local hooks = {}

-- 0x0800F024 (r2 0): put on what `form` wears.
function hooks.start(me: Object, form: FormDef)
    me:put_on_form_overlay(form)
end

-- 0x0800F088: take off what `form` wore.
function hooks.finish(me: Object, form: FormDef)
    me:take_off_form_overlay(form)
end

-- The status reset (`sub_80144C0`: the weapons too); 0x08011A94: the anger over, the mood back to 0x80 (BN5's
-- setter leaves a mood of 0); 0x0802C126: the soul's charge state cleared (and AIData +0x36 and +0xF0, the
-- computer navis' AI's, which a player's navi doesn't run); `sub_800EB08`: no longer invulnerable.
function hooks.settle(me: Object)
    me:reset_status()
    local stats = battle.navi(me.alliance)
    local mood = stats.mood
    me:end_anger()
    if mood == 0 then
        stats.mood = 0
    end
    me:set_navi_state("no_charge", false)
    me:clear_invulnerable()
end

return hooks
