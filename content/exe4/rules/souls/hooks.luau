--!strict
-- What EXE4's soul change (./change) and revert (./revert) share: a form's start and end (0x0800B7E8: the overlay of
-- the soul, its form's own, by the table 0x08018068: FireSoul's and NumberSoul's, docs/design/exe4-map.md §18 item 74;
-- the engine's form overlay routines put on and take off what a form wears), the identity he is taken for, and the
-- status reset that settles MegaMan in his new form.

local hooks = {}

-- 0x0800B7E8's spawn: put on what `form` wears.
function hooks.start(me: Object, form: FormDef)
    me:put_on_form_overlay(form)
end

-- What MegaMan is taken for follows his form: a form's identity, else his navi's (EXE4's souls have none of their own:
-- the chip icons over him sit by his navi, 0x0800B9C0).
function hooks.identity(me: Object, form: FormDef)
    me.identity = form.identity or battle.navi(me.alliance).navi.identity
end

-- 0x0800B7E8's free (AIData +0x48): take off what `form` wore.
function hooks.finish(me: Object, form: FormDef)
    me:take_off_form_overlay(form)
end

-- The status reset (0x0800E05C: the weapons, the soul's own routine) and 0x0800C5EA: the anger over (flag 0x200000,
-- AIData +0x2E), the mood back to 0x80 (`sub_8015BEC`).
function hooks.settle(me: Object)
    me:reset_status()
    me:end_anger()
end

return hooks
