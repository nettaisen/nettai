--!strict
-- Otenko's statue's identity: NameID 0xCF (actor record 0xCF: AI index 0x21, EXE5's field objects'). Django's
-- navi (../django/navi) tells Otenko on his side by it.

return new.identity {
    class = "field_object",
    version = 2,
    ai_index = 0x21,
    look = { sprite = asset.sprite("otenko"), anim = 1, palette = 0, shadow = true },
}
