#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

enum class rs_Suit {
  spade = 0,
  heart = 1,
  club = 2,
  diamond = 3,
  joker = 4,
};

enum class rs_TexasType {
  no_calc,
  high_card,
  one_pair,
  two_pair,
  three,
  straight,
  flush,
  full_house,
  four,
  straight_flush,
  royal_flush,
};

struct rs_GinRummyCards;

struct rs_PokerCards;

struct rs_TexasCards;

struct rs_PokerCard {
  rs_Suit suit;
  uint8_t number;
};

struct rs_CardBuffer {
  rs_PokerCard *data;
  uintptr_t len;
};

struct rs_Counter {
  rs_Suit t;
  uint8_t n;
  uint8_t bucket[14];
};

struct rs_TexasCardBuffer {
  rs_CardBuffer cardbuf;
  rs_TexasType texas;
  uint64_t score;
};

extern "C" {

rs_GinRummyCards *rs_GinRummyCards_new();

void rs_GinRummyCards_free(rs_GinRummyCards *p_pcs);

int8_t rs_GinRummyCards_sort(rs_GinRummyCards *p_pcs, uint8_t *p_out);

int8_t rs_GinRummyCards_assign(rs_GinRummyCards *p_pcs,
                               const uint16_t *p_data,
                               uintptr_t data_len,
                               uint8_t freeze,
                               uint8_t *p_out);

rs_PokerCards *rs_PokerCards_new();

void rs_PokerCards_free(rs_PokerCards *p_pcs);

int8_t rs_PokerCards_assign(rs_PokerCards *p_pcs, const uint16_t *p_data, uintptr_t data_len);

rs_CardBuffer rs_PokerCards_get_cards(rs_PokerCards *p_pcs);

void rs_CardBuffer_free(rs_CardBuffer buf);

rs_Counter *rs_PokerCards_get_counter(rs_PokerCards *p_stu, rs_Suit s);

rs_Counter *rs_Counter_new(rs_Suit s);

void rs_Counter_free(rs_Counter *p_counter);

rs_PokerCard *rs_PokerCard_new(uint16_t n);

void rs_PokerCard_free(rs_PokerCard *p_poker);

rs_TexasCards *rs_TexasCards_new();

void rs_TexasCards_free(rs_TexasCards *p_poker);

int8_t rs_TexasCards_assign(rs_TexasCards *p_pcs, const uint16_t *p_data, uintptr_t data_len);

rs_TexasCardBuffer rs_TexasCards_get_best(rs_TexasCards *p_pcs);

void rs_TexasCardBuffer_free(rs_TexasCardBuffer buf);

} // extern "C"
