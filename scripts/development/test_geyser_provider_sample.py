"""2026-10-29: checks for minimal Yellowstone slot decoding."""
import unittest

from geyser_provider_sample import response_type


class SlotDecodeTests(unittest.TestCase):
    def test_slot_update_with_unknown_fields(self):
        # Update.slot {slot: 150, parent: 149, status: 2}, plus unknown field.
        update = response_type().FromString(b"\x1a\x08\x08\x96\x01\x10\x95\x01\x18\x02\x78\x01")
        self.assertTrue(update.HasField("slot"))
        self.assertEqual((update.slot.slot, update.slot.parent, update.slot.status), (150, 149, 2))

    def test_ping_is_not_a_slot(self):
        self.assertFalse(response_type().FromString(b"\x32\x00").HasField("slot"))

    def test_processed_status_defaults_to_zero(self):
        update = response_type().FromString(b"\x1a\x02\x08\x01")
        self.assertEqual(update.slot.status, 0)


if __name__ == "__main__":
    unittest.main()
