from uniffi_futures import always_ready, void, sleep, say_after, race_say_after, new_megaphone, say_after_with_tokio, fallible_me, MyError, MyRecord, new_my_record
import uniffi_futures
import unittest
from datetime import datetime
import asyncio
import gc

def now():
    return datetime.now()

class TestFutures(unittest.TestCase):
    def test_always_ready(self):
        async def test():
            t0 = now()
            result = await always_ready()
            t1 = now()

            t_delta = (t1 - t0).total_seconds()
            self.assertTrue(t_delta < 0.1)
            self.assertEqual(result, True)

        asyncio.run(test())

    def test_void(self):
        async def test():
            t0 = now()
            result = await void()
            t1 = now()

            t_delta = (t1 - t0).total_seconds()
            self.assertTrue(t_delta < 0.1)
            self.assertEqual(result, None)

        asyncio.run(test())

    def test_sleep(self):
        async def test():
            t0 = now()
            await sleep(2000)
            t1 = now()

            t_delta = (t1 - t0).total_seconds()
            self.assertTrue(t_delta > 2 and t_delta < 2.1)

        asyncio.run(test())

    def test_sequential_futures(self):
        async def test():
            t0 = now()
            result_alice = await say_after(1000, 'Alice')
            result_bob = await say_after(2000, 'Bob')
            t1 = now()

            t_delta = (t1 - t0).total_seconds()
            self.assertTrue(t_delta > 3 and t_delta < 3.1)
            self.assertEqual(result_alice, 'Hello, Alice!')
            self.assertEqual(result_bob, 'Hello, Bob!')

        asyncio.run(test())

    def test_concurrent_tasks(self):
        async def test():
            alice = asyncio.create_task(say_after(1000, 'Alice'))
            bob = asyncio.create_task(say_after(2000, 'Bob'))

            t0 = now()
            result_alice = await alice
            result_bob = await bob
            t1 = now()

            t_delta = (t1 - t0).total_seconds()
            self.assertTrue(t_delta > 2 and t_delta < 2.1)
            self.assertEqual(result_alice, 'Hello, Alice!')
            self.assertEqual(result_bob, 'Hello, Bob!')

        asyncio.run(test())

    def test_race_say_after(self):
        async def test():
            # `race_say_after` creates 2 Rust futures and races them.  The
            # resulting future will be resolved by whichever child finishes
            # first.
            result = await race_say_after(500, 'Alice', 1000, 'Bob')
            self.assertEqual(result, 'Hello, Alice!')
            # The Rust future has now been polled to completion and should not
            # be polled again.
            #
            # Sleep long enough for the second child future to finish.  Python
            # will get a waker callback, but should not poll the future again.
            # If it does, this probably crash the program.
            await asyncio.sleep(1)

        asyncio.run(test())

    def test_async_methods(self):
        async def test():
            megaphone = new_megaphone()
            t0 = now()
            result_alice = await megaphone.say_after(2000, 'Alice')
            t1 = now()

            t_delta = (t1 - t0).total_seconds()
            self.assertTrue(t_delta > 2 and t_delta < 2.1)
            self.assertEqual(result_alice, 'HELLO, ALICE!')

        asyncio.run(test())

    def test_with_tokio_runtime(self):
        async def test():
            t0 = now()
            result_alice = await say_after_with_tokio(2000, 'Alice')
            t1 = now()

            t_delta = (t1 - t0).total_seconds()
            self.assertTrue(t_delta > 2 and t_delta < 2.1)
            self.assertEqual(result_alice, 'Hello, Alice (with Tokio)!')

        asyncio.run(test())

    def test_fallible(self):
        async def test():
            t0 = now()
            result = await fallible_me(False)
            t1 = now()

            t_delta = (t1 - t0).total_seconds()
            self.assertTrue(t_delta > 0 and t_delta < 0.1)
            self.assertEqual(result.value, 42)

            try:
                result = await fallible_me(True)
                self.assertTrue(False) # should never be reached
            except MyError as exception:
                self.assertTrue(True)

            megaphone = new_megaphone()

            t0 = now()
            result = await megaphone.fallible_me(False)
            t1 = now()

            t_delta = (t1 - t0).total_seconds()
            self.assertTrue(t_delta > 0 and t_delta < 0.1)
            self.assertEqual(result.value, 42)

            try:
                result = await megaphone.fallible_me(True)
                self.assertTrue(False) # should never be reached
            except MyError as exception:
                self.assertTrue(True)

        asyncio.run(test())

    def test_record(self):
        async def test():
            result = await new_my_record("foo", 42)
            self.assertEqual(result.__class__, MyRecord)
            self.assertEqual(result.a, "foo")
            self.assertEqual(result.b, 42)

        asyncio.run(test())

    def test_cleanup(self):
        async def test():
            await always_ready()

        asyncio.run(test())
        live_futures = [o for o in gc.get_referrers(uniffi_futures.Future) if type(o) == uniffi_futures.Future]
        self.assertEqual(len(live_futures), 0, "Futures still alive after async function has finished")

if __name__ == '__main__':
    unittest.main(defaultTest='TestFutures.test_race_say_after')
    #unittest.main()
