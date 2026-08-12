import {
  Type,
  ThinkingLevel,
  FunctionCallingConfigMode,
  Modality,
  MediaResolution,
} from "@google/genai";
import OpenAI from "openai";
import {
  ChatCompletionAssistantMessageWithCacheControl,
  ChatCompletionSystemMessageWithCacheControl,
  ChatCompletionTextPartWithCacheControl,
  AnthropicMessageCreateParams,
  TestCase,
  TestCaseCollection,
} from "./types";
import {
  OPENAI_CHAT_COMPLETIONS_MODEL,
  OPENAI_RESPONSES_MODEL,
  OPENAI_SOL_MODEL,
  OPENAI_REASONING_NONE_MODEL,
  OPENAI_NON_REASONING_MODEL,
  OPENAI_MINI_REASONING_MODEL,
  ANTHROPIC_MODEL,
  ANTHROPIC_FABLE_MODEL,
  ANTHROPIC_OPUS_MODEL,
  GOOGLE_MODEL,
  GOOGLE_GEMINI_3_MODEL,
  GOOGLE_IMAGE_MODEL,
  GOOGLE_TTS_MODEL,
  BEDROCK_MODEL,
  BEDROCK_ANTHROPIC_MODEL,
  VERTEX_GOOGLE_MODEL,
} from "./models";

type ChatCompletionAssistantMessageWithReasoningSignature =
  OpenAI.Chat.Completions.ChatCompletionAssistantMessageParam & {
    reasoning?: string;
    reasoning_signature: string | string[];
  };

const chatCompletionCacheControlTextPart = {
  type: "text",
  text: "Use this stable reference text as cacheable context.",
  cache_control: { type: "ephemeral" },
  prompt_cache_breakpoint: { mode: "explicit" },
} satisfies ChatCompletionTextPartWithCacheControl;

const chatCompletionAssistantCacheControlTextPart = {
  type: "text",
  text: "This assistant prefill should remain cacheable.",
  cache_control: { type: "ephemeral" },
  prompt_cache_breakpoint: { mode: "explicit" },
} satisfies ChatCompletionTextPartWithCacheControl;

const chatCompletionAssistantCacheControlMessage = {
  role: "assistant",
  content: [chatCompletionAssistantCacheControlTextPart],
} satisfies ChatCompletionAssistantMessageWithCacheControl;

const chatCompletionSystemCacheControlMessage = {
  role: "system",
  content: [chatCompletionCacheControlTextPart],
} satisfies ChatCompletionSystemMessageWithCacheControl;

const googleToolCallThoughtSignatureReplayAssistantMessage: ChatCompletionAssistantMessageWithReasoningSignature =
  {
    role: "assistant",
    content: null,
    reasoning_signature: "dGhvdWdodF9zaWduYXR1cmVfMTIz",
    tool_calls: [
      {
        id: "call_123",
        type: "function",
        function: {
          name: "list_collections",
          arguments: JSON.stringify({ database: "mydb" }),
        },
      },
    ],
  };

const openAIMultipleReasoningSignatures = [
  "gAAAAABqfNlzeppqdBPUwNBuyaHIEquF-4ImX9Ixm-r9cbTSdWrIF3Q7vLg-l3U4ZmFQXpJa734dWa4FTe4boCJlGOlkJXJ1vCA8Tbtbi_Xe_BlnbTBUjw3SwFg61ZTFuhkL3_WOP7x3-7prCGbmAAhqn7zAaC2esGvQjPqR9CE-xMewzQXpDOtN-zW2dp4uZBO4o1vzIylfmgLcTW7atHX4S6ckpyL4Oh_nD48nNiJDhH4Oo03no0DuIs2JfyB3fLak-Jd5yv8y4iATfyq-KIdg_Wy03A6w5rb6jZbWVdzE_XnQwJ6Fc2eFb-RotacFycn_Or0RXgPTaVOpu_1rkGEBtPtGPE5e0ksrFKkECy02hxCECIryjX47SptDIbeIkeukMGS2s8tGHyCnF8iNrArixUCpfLxvUMRKAaMjy-pZd-FiDnh-FKnzJ0jAe00lLq2eQP4XBlrZWPryEpAW1UDCaq_wnEfVt_zgib-WNEDDY6qEvKCSURTWL_smBFQuMPMJj84BzEjUgn5ooVXxni8aWe1bcND6KH0dtNgvOxY7OS_SgAURGn-yFsPa6xb-LoYPtNrt65U7UJsP77ihgW9Xux6lYMsx-6ZmEeGzldPAZEiu7ao_eLMnSFp5ytJpnLqRGMlqPRR4d-nGMd6XUrzf9nbMD1MRbFub2lTZ2p9fPt-hZnnD4DuBbdgdW8YoMUnwZV5XiydagxyQIBJRmQRYctno0_NwxPh7_UI3KY5xwlGNIsChhE9YyOu6j2comye66a9LFP7C_Tltm6sr9dWJY9SDG0tdyZ5W_0-1URY8fq0sMB1F372RyTvLBmLswDd_qR6gpuuGl_B2-yrqn5buyKBMczokUCN3mSYCUmh6V_-xG9YFxAvovDczTQsnPSOucU9XJIei6N366tuwsDNIbfHRp31yp82ltpzoTDXvhDw-F74SSVVQ_GNDo6VyPOuUB6yfgOMZMl3mCFVjiywMBNhqjRY2cqROIeXyUVhDKi3vN_80NJo3HGkidfLJvXYCz3x8uvuJrnyoY1quWZnX5ommovW_sUGUU1wLehecmScUaSLpUN0si7x0M5mF5eSpgcUit12hbZYwNo0_LQDQ46xCNILLttEeb41oL3IeNYsi9rfcg8yWVCKBE2nT6qOFZHOe4wDsDD2XrUw7Vv15v0M9dhP-HvJRKYDhvP3j0xeAKwGAiTxwbnud0Jn-pIrOsmk8OBYqhL6yyEkFm1qezgOwATVANRazkvhpbAEJSiIrw34bKjvAwGAiYZjuIZVW2BB-z1s7SdNqtPm5i00-pQ1NWuN1sw==",
  "gAAAAABqfNlz_LB-r4sk6UdWgcOGivPVP8a36glTojypQM7JUqu8j5qkglCv-0OEoatvVgwXv0ZWYZ-R5PUySDjT8Fum2Ta_f7whcgiNGgNeldgOLhLtkNt_hQeOqT8QV-zMUlLbm3GNm_KMmVxMU34YjPBx9FYpTcv8dZV29f4hRl3TK6FGDRsbs6NoE8jh0N4cKW4BvVdRU-SMyS7k1N71DKJo57uMh-BEm1cJrexsgN5LHLPrJtdi-wPuWCDYS1atD6MwAftS_mZ8Pve99Zq_3qVuGblrBlI6QhRVl58-8qO4YQDCKjuCazvmsGVFs8Mr2TqBkFncoFPZrVsPdbbQHi9AgTSaMFx0zpkuwMvfE1NtwwEGcR6j8rwNY_nE4kUZ3ObbboUgM7WbwEMI-STbznIXPCJXbO7ZE26HC4bzV_cKHxvat0dq1h8qf2zJIy9TH42eXb-1RWv217wHRIRfWYSV9D6nuFBgNvAMoIsaEhn-4eAmyGsPIHxKktUm-Tr-V4r5FFwQe3O00GAezQJSQnBal1czQQk4btyRzxJY26Cnkhyo1QDXlk3GDMfbk_0zbrpH8FT8AZllbUnBBWKS16kjouLm9ikcGS08_VaCY7Q72CpqQwbwj70c3zkQnigmFoMJuuOY6okqTv-IJRGgaKJuJ_NwqJWIJHeU5DkCb4Jr2Lwnc0aMKHIlEk-UikMzDIw7dYQJFdmQUrnLJIK3TeUZNzE8SMIGM_QT6FSD6IOCgdwQlbc4RuRdmywNKsNmVCwJKtEw4Pya5knZ3tr7TEViX1wpqzSBMApfBf8MVxSJbAEXS-EFm5RAuM9yeNkSjSlvoQH4_ZaJrQCtkxcVL_tApnlOTslMHMYn37URwIXXQn1QwKH0_0yFfYlxpBbbPAp7uEMrqriHUjjiaTXtHTvE_f3vcnfIBDhFDMcGGst2R2F07dk-qJfb4Hn_TqH17gGW13Hqd9It9MPVxLjqs4KrWZejuq_ay66n5yJXtGrlpvZ5Qt-7yW0jHIyNcmOwkbVbWawWfOXYyk4vINLmyjhopKvzKg==",
  "gAAAAABqfNlz4Q3g1e2MWX-hAgkKBfWnIO8ovWjZYuRt2K0g4XQ3gwXu1V_jjIqeWnLZZz-G3M_-y7sUlC4A5z70cGaUKmfswpQtjfpk3be5IRHXkxo7gUvX0sJBFbAyO6nzxhoLpkM7oR3eABGU74JgswXpYRZo1U7AEK39lJTq6eXarkbSLB0J8TnhE5vEJDekLlWTlt1-IGK0tEYL81jtkkZBe83-e1bMbv-SbNpJZLtNT-eYhO-Jj02v5JwgLbhK9Rg1PCM-_50sphqhMZwWpRPQbqpqlGp88BJ635DQzAsSckqJ7_isxgnvWzpj98-7XHj1BEuZoNBwkom8FxgsZ1p3n0Q18hg1bvWjCS0b3jFElYDjJ-mmiB0jk0xMxIKS2hzConS31C8LmNWWJMScBSjbpqkgq6BrBPyUNUYe1iD3iwJlD3lkeBs0_YdM6KLrPiRLRqLtwlw1cOUEVCZM_Z4n7_oV78hd9J4LI3qYcr0n-eKdvwlcxZ-PBthnmPfId1da4V69Txs091l6L2Dz5XsnxQkUZ1NvyDN6gwjmXVoKNbV0zBB3qKleO18eHlKS9KTKz62to5AW26VXpdO9tcN18_01a8wtoT-vPuuAc1Mw2fxJt60DFHYTlkduweGs8JHNTvIQAUBXLrIFWmaBdH_oQqmMyUIoWq_0b02wN-3rcxzh91Ss0G92kq7TxRR9iD5qIGou0iTuQO2JzwgbIsIM8ELH5iy199NXrEu5BIbsmg2o5gN_KPnNULLaasJyxe38OYkGHWLKs_Babah65iURV9Np0LrDHWauTjORYJpcVKq351tbWPM1mjLRdgwjyM0yeaxvEI40KcEzGJWlJDcmVlm-k1KuEtYotGI8Dt1o5mdIbpCRZp-tyx72hGxPHe7smOpslChT_nIHJAuVWgxgYTvuLm0vjMn3HS_OAgIm2CsMqGfcDKEqEhfTXjDqyTVKfM0yBf1YGY0tsuGuTLGDS2rghUZNz8-wHPVEFava9xtGiQupoVMB3_B9i_L2WHzqg18Gi53wq9ffp7CIaYVgL538LYpP-cimJvp5e14heAG_fMJeeGTCarKlcsALJCbywssU0LoE7QBrwvWKEc_j-l2v-uax7FNHyuAouiFYz1FWpq-Q5WYL1ALDDLvuOWVRVlckenmW1W-KH4PJCuzxLNcn3wcBp7maJ_kovD86T-VDOqX3o5VJlhioMjHQmwb9VMAWC2Y2QbawsxEwZ742alSSTcUMPjbnCZDQuHiinjsKgSWoJBxS3sIFRU5NA6X33IRyGtuSUZtmD0c1Ex246uz36sSpU9VKl6AJlX5VcYLkz3jl3pf4ZNrtbmorqcgta-PqlJ2MHQE0TaGdrZ0960Mzz8Sp2p1k5tiT30heTjQqiyFTnTbeA3NTIEpYg3aMh5_Y",
  "gAAAAABqfNlzO5CGfR6hGpnCM1kFTpV855Vg4B3EvevRNU72bHE7pZN09jH112WwjxdcICRUeWKhj9sOLC-1wNRjSx5_ymrpdr6hkVsIAzgYmfXHsGM0mKmu-JMi0SQ0ULqv6dVOMM1u8dXoXwIc7DZ9nAp9ZXi5wxd0fJD1uWZqdVofWJ2DYPv1aX_nQbBmqVDppnPSY8VZdSlwEG8rUf1xeVHh5iHzc-3fFp6jmSA767fsdUZXcPrBhr4DJjOeqWZS7nyR7u59dxpHaCwwpI6i0G6s8e52honLowyQUMCwii6-GqRxq3vop4L1aqcMqXn-5885LwD8GugRn070ka8axF1eXJj9IB4VNSIw15QaLSNSLytCxzHObs5SIPi5ciMgf_MA3-rpujSqadNcYRU_Xy-t90mf5-VKg6LxrX3oAuWvJJHya8DWgwz9JzzkXha7B1jyZGsGCmcvtAQ1EznXF-FNVN3lLhm0tXI2ZO_0zv0JF3vA9cILEhfkQgA1_WKqc3IWUuhfQVBmwcdyVUKXwUIP3sp5QoSmxqizRIkWrfMrwVXIQUImUBUEloiZkYgpdjDjm7sknPZLzpKhCD8IsbfaiQvoDdFUIj2Xe9qpbFLslZuX90CE1LfYLZ2ASq-t8RWpBa8ZcNxVJK9bZR-KR7zA7Tk2E8UwuQeWWp2mm9sTlZLY2S5yDSFKmETqnSTFlCWqjcgglWs_WF6vf2ISyqLCG0WoaBKXQ1EqkL45pAcjSXDeJEd4WW-P4iSSnihdHW6ih0ZJ_d-2f3QK3QRjYvX8tsWQpWxB4PyM8F9DbIJ-QIhGXY1TKBKuVsjNWHZNempyjp9zMUejGiIPBmqwEViMkq0yg6YhP5xr2XC6Qv7DRzfhASGCYoyODS13mNsUTe7yQxI_ql9y9WhpymEQs5oe_0XQ7fRHdMdVFh39n1y8nmLJCD9gcn5ju27moS9mtrjfQ9uNsp_rpsX7HRvPg4Fy4_PqMkqAKgOR7czVm-VgDasnX5Im6vefUE7azaZCaztXOR4N",
  "gAAAAABqfNlzlroeYISIA_aeQrTG198nzFmNy150chXnnnSTfJInKwuTMDrgEUWxH8r6hpPo_TQ-kH46zvjkgwS-zERU4zYIPZFplyHGzYg5bCADqtw2BYRVT4SJNzbm281vykEJEgldB4-fM5CqCMACJOIuUB2QbcHd2SLqmjuxxV3CZ3anjKBrr004aD6bYE8uLIH3T96LBtViE4PbtHZB4dCP0scwccf8GzUdekVEbSJ6ciihb-BdhilkHMl1KrejVplYjIMnBAQlT4eMqc7siKelbe91D72ni4RxDZ0LCsmH19EAn7T-1o-7uMDSfGJQt9L26zj5TJQf8S8Y9KDNG3-1dZ9vx-5MA5DL5gwsR4Q2ktIE8dcuozHOE09KZkTUYGR04r7-S3q14kcJjumkPcfuZaVEWGKYFcsYubyWzRpQlyRDwhPSAw1JkirhndVqBATmA6Akzmcy6I6Ga6QKpbm76EOeieBgHXLmIPZKUm_wOWPFEUXdQKZpqLEPj2J1gH6kMf8_Q_-oD5MxYlQ5coyRwlBlCSiWXykJV5T7fTsHF-208noYZlWw1EdgC19zMQd6tbfIa8m4aAdKJN7Cs8RElAamo1R0Fgkt0K28XQZ8jMXHekCb-hsS-v2TMZwzaUfu-zZEj3nHHde2lkqjMpb-Fx04yzGqZfVC3hZJ-6oaaoFW1vIteeiv8nPlCdiED8oYKv7NxypI-mT8hjZbN4N9EbI5mKNMydT3k-xq8b7lJfzEpbnTqEvoakjxpUCPX0jsG0AxVAJEeJVd4fJH8hJ8UDoCsqayn9K4-hcsU-_2lzEEB-MRMfT3X-EckTvmcNqapkPMD1ccV_e2Lqov16dEkhcLkCaHx2GxafgEcK8j1DGxcF8p0TUEXFsiFRmer2iCflwDWtr3aneiJwkRsGdZNEVY69Et7sY1pYld-j_yDKXGb7ezuCFtDC73GHDGbYJr2hjki8rUOGtj_WbajuslNqTJCbxx3wgLVNHLvT9KceBNKUBccGrsaqKt-G_wqZ17QkXnvq_gyZcy0kUsKHKIxd587oYSoLwuJVDRIK3f8a-NZzQcFO80BT_GeSy0oD-Q22BaKuk8qOWsfBek59Lqhspuq2WwvWbIwBU43V5Ys11-MhTCXudZJzVVM3h7Fs41X8pBg6y-io28Yk9uGEu6No7CzrHTHDbQyFmNeyh3AaDYj4m8vJ-A1RwaAhkcafeRQU4wq7vUbD9DPPK4odb6NDPUNVUQ64EXa2UqoJT56_Jc5RouBu5HmBUHDPqXJX66ZrQn36Tagq1Kos43M9kiCZiNfCCgKXZalSHb4mC22B-kpZ7krHoux8WFoiKL1_98hvMxgLb214rqbds1FUDgXDfwCBiB9KC2cM_UD_H4sFEmG853RxodN6ZPBNrQiXXxoJg6FvdAHa1-21XtM7ISF_2cnL8ZDjYTugCfN7tA1ZNCiOzLLh6IiE5gxTOC2kGjvUhSG8LePtGoPKg1bRT8TJ7riHPyOfz2maAyEbznK3TdE3oOlNic1N2hG62Fx5M0nVWwchp5eBWureJ9AEMUwmwN1TjNBQj4kMi4j0fJgPXGBCEh0ZC5Sp1rvwNhChofnNoAoWltAq0T9fPrqypmlRv_Xo4ZJwVcHCPvILBqKGgvNPXzQuccB9A7LwDBph-HnYfsOGnna9OJeTlPmXuQUEeDwIINHSHMIngYlHn7LQaUg_mNz528CrF0cdpxJPAW9MEleLgg5ykm2eaQnbvvJYuk5BTpK0TZnDvHDKpil7Ga5Qd3N_xg4liGfv3BLqGyE37bW04g5MdhPxF3L2FKPq6ooAni6ByOb7O6-YV6h883MrifSI1JfGyKhqAHFi_TrCuFs_PJz_p1SVv2wRxy1cJjX_2YKqtiB4mebDdGkr_IXC2s2x40grSXrW7ZfGG7HORLvVRHQSe7bose7iaajZv55rx5LixnjG3JA_ZF89NSlH-WQGHoAK7Q8M3afYm3tiE5k2LKDHgVUVg0BuDZwYpmMaN5gmspE3oXVlPJSV-cLnxydfcvAHaoHNGBMr2aik-pZXaoRsHsjWNOJbmu6aDY2UgZRzBIHwcLbdqNOCA-VuZ0JhrXm1SdRZ6dDk8EvbPJOtgwE_ZNnwj1xYulJ31wUKCiahNfjyR1DIcZI7fFxLgOnm28VeA3Ugh39KyY3e2KsSB0ZC2fAAyk6OARHybDEjVQhQupjKTcseiiWTryYusbMgKpiJ3Q_-gjLGhlta0JrYmQeQ3yepXC2Iix-o9KaZXIXmDpi83_GeU19Dj92dRcc4GtyNuSZ6YcXsqy48Euqu0YY1s03M-gi1PDEFJ_wJ6oIAKooBlDVNU9n_7vjk5Ltu7hrV9KSPhM0QEvBPYtPsmgVQWULPhs-VO2nQ4zMjNxEfvRND-0jeQx-nU1Do5mTErh5Jmz3yD1iKWQ_OLdMd_dq2cH7PELzC9wje2kTrhBPQYvwXT3yIznYxFMVpBP7Xr0Q-Vp-2chxZCuKDE4rTq1S0eyrNjW5kO8WZ0CJq8PmlpR1ACPDkJgRUBzd3OhbiDlj2gq6UmurEFQyikl0EWfsbL1PrN8nrEOuesUlI4yunyoAtkIqqbuu8VUc9LsDZd2QeMhUdFSOTXi3qVglaHzfDnxsbMSlQONMXyMecKHfuLAL6JsaXXH1a3hntc5MiCNbxdgIg8B3CRip1n9viJwQRiWnwdWkscqobwNctdqtT8Ttgq5ebacPwOh9GNqY2vrrX8g9tsi2H_lMoad2P3-xmxohNgcyFu7ID3xqvyeTn_iTeXX0UDbZuv9j7sqtaCKOIj5m8PFcAjkLWmF-gJYQdd0M4vMvXEPt_cA9mo4CeKVe7iuRVAm1G4kBSfLU1QqHf--u8DS53PutD26ezLBp1w4tch_zegkgl96t-w7nKWK1i7dvNqt33_W31XGKI5n38dfrFwKyqp195vebf7gRSQHf4kvCV-hdEmzHmblTCHPSvNZbcq04T61nsY8STQorHLIWbRZ2FcEOKOhGY_6cNjlvChETS4_fCuc6-bBE22dHxCdGCMNv75zzfFKg0G4FjINQuVWXTUk2hBkhz1RPyFR1rOSxTokirbfchizsaDXw1aL9wbICAzwkeezSXEa91zQoYhZNjtc8FzH6G7ZgtK61aOy4ak8biWVO-sVKbM4_286jyA-3MgL7ZL6NawPPQAAkxWavuAXMOTBpf7vrAw0WQH2OUElg1D-Gou8TevGW4RKUL5bjo9Nnes-j79vwHJ4ZcBnhdnlXDM4lVi6IU57cfqQM4vd6H7r-EJp8U82_qoiUUMZK6CMFCY78HV2y8JyV17IPHgSmwZFvwKoJB1Pu5l33EvcWK9MQ3DacpvNCn5VaP6ooatyCUSXHjOaJnKSwu-9ZJC7dzZhXfT6a9oTAllTe3mcRWquSOpFz5ynnNS-rmUstJ2gIDePWWpvortk-n4_9ymT39uYcJoyuPeOWpZFLUPXQu0kfT3YTJnN6-uLdS3Q6YxKoXUxFMlXq957RZDqZ_SHxtpUn6Bv0zlzptyO79dYTrPC1GWyuy69mCVzdyAs1crgGEVa6-upOAdNNq5pyouJooIoQcGcSjGEfYrQuNKrs0Om6Z48SqftWlDkLCDE8SURIo4p5be-jCZ-PWgbhRVqC7QuAtjEQuEeWWlGFgVBDdKWX1nQanCTUCzI-iC_MaYpuy-230-9D7q6yvRkpZ7jOVHjoVSt1uols5UjJWd1KZrWyk5QbI2HnXswnw5898OYBjbX6TTn9phRnJyqy22ziYFT_T2_G2QxjI53ckuM_-8TIEr5BssQeOrKt9NEbwzD66X0QFraPiM-iCOzL50PeN51FfguqMZ-S60TjFrPxcjTkA==",
];

const openAIMultipleReasoningSignaturesReplayAssistantMessage: ChatCompletionAssistantMessageWithReasoningSignature =
  {
    role: "assistant",
    content: "",
    reasoning: "",
    reasoning_signature: openAIMultipleReasoningSignatures,
  };

// OpenAI Responses API and Chat Completions API parameter test cases
// Each test case exercises specific parameters with bidirectional mappings where possible
// Note: temperature, top_p, and logprobs are not supported with reasoning models (gpt-5-nano)
export const paramsCases: TestCaseCollection = {
  openAIMultipleReasoningSignaturesReplayParam: {
    transform_targets: ["responses"],
    "chat-completions": {
      model: "gpt-5.6-luna",
      messages: [
        {
          role: "user",
          content: "Perform these steps in order: (1) use web search to find the current capital of Kazakhstan; (2) after that search completes, use web search again to find its current population; (3) call record_research with the country, capital, and population. Do not answer before the function call, and do not combine the two searches.",
        },
        openAIMultipleReasoningSignaturesReplayAssistantMessage,
        {
          role: "user",
          content: "Now provide a concise confirmation that the prior research context was received.",
        },
      ],
    },
    responses: {
      model: "gpt-5.6-luna",
      input: [
        {
          role: "user",
          content: "Perform these steps in order: (1) use web search to find the current capital of Kazakhstan; (2) after that search completes, use web search again to find its current population; (3) call record_research with the country, capital, and population. Do not answer before the function call, and do not combine the two searches.",
        },
        ...openAIMultipleReasoningSignatures.map((encrypted_content) => ({
          type: "reasoning",
          content: [],
          summary: [],
          encrypted_content,
        })),
        {
          role: "user",
          content: "Now provide a concise confirmation that the prior research context was received.",
        },
      ],
      reasoning: { effort: "xhigh" },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  openaiServiceTierFastParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_SOL_MODEL,
      input: [{ role: "user", content: "Reply with hello." }],
      service_tier: "fast",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  bedrockDocumentCitationStreamingParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: null,
    bedrock: {
      modelId: BEDROCK_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              document: {
                name: "gateway-release-notes",
                format: "txt",
                source: {
                  bytes: new TextEncoder().encode(
                    "Braintrust Gateway supports OpenAI-compatible streaming over Bedrock Converse."
                  ),
                },
              },
            },
            {
              text: "Answer using the document and cite the source: what streaming route is supported?",
            },
          ],
        },
      ],
    },
  },

  bedrockGuardrailStopReasonStreamingParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: null,
    bedrock: {
      modelId: BEDROCK_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              text: "If a configured Bedrock guardrail intervenes, return the guardrail intervention response.",
            },
          ],
        },
      ],
      additionalModelResponseFieldPaths: ["/stop_sequence"],
    },
  },

  bedrockAnthropicContextManagementParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "hi" }],
      context_management: {
        edits: [{ type: "clear_tool_uses_20250919" }],
      },
    },
    google: null,
    bedrock: null,
    "bedrock-anthropic": {
      model: BEDROCK_ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "hi" }],
      context_management: {
        edits: [{ type: "clear_tool_uses_20250919" }],
      },
    },
  },

  openaiPromptCacheKeyParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Summarize the cached policy." }],
      prompt_cache_key: "policy-cache-v1",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Summarize the cached policy." }],
      prompt_cache_key: "policy-cache-v1",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  responsesToolSearchInputParam: {
    "chat-completions": null,
    responses: {
      model: "gpt-5.5",
      input: [
        {
          type: "message",
          role: "user",
          content: "Find the available tools.",
        },
        {
          type: "tool_search_call",
          call_id: "call_tool_search_123",
          status: "completed",
          execution: "client",
          arguments: {},
        },
        {
          type: "tool_search_output",
          call_id: "call_tool_search_123",
          status: "completed",
          execution: "client",
          tools: [
            {
              type: "function",
              name: "search_code",
              description: "Search code.",
              strict: true,
              parameters: {
                type: "object",
                properties: {},
                additionalProperties: false,
              },
            },
          ],
        },
        {
          type: "message",
          role: "user",
          content: "Use the discovered tool list.",
        },
      ],
      tools: [
        {
          type: "namespace",
          name: "search_code",
          description: "Deferred code search tools.",
          tools: [
            {
              type: "function",
              name: "search_code",
              description: "Search code.",
              strict: true,
              parameters: {
                type: "object",
                properties: {},
                additionalProperties: false,
              },
              defer_loading: true,
            },
          ],
        },
        {
          type: "tool_search",
        },
      ],
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        {
          role: "user",
          content: "Find the available tools.",
        },
      ],
      tools: [
        {
          type: "tool_search_tool_regex_20251119",
          name: "tool_search_tool_regex",
        },
        {
          name: "search_code",
          description: "Search code.",
          input_schema: {
            type: "object",
            properties: {},
            additionalProperties: false,
          },
          defer_loading: true,
        },
      ],
    } satisfies AnthropicMessageCreateParams,
    google: null,
    bedrock: null,
  },

  chatCompletionsAnthropicCacheControlParam: {
    "chat-completions": {
      model: OPENAI_RESPONSES_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: chatCompletionCacheControlTextPart.text,
              cache_control: { type: "ephemeral" },
              prompt_cache_breakpoint: { mode: "explicit" },
            },
            {
              type: "text",
              text: "Now summarize it.",
            },
          ],
        },
      ],
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: chatCompletionCacheControlTextPart.text,
              cache_control: { type: "ephemeral", ttl: "1h" },
            },
            {
              type: "text",
              text: "Now summarize it.",
            },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  chatCompletionsAssistantCacheControlParam: {
    "chat-completions": {
      model: OPENAI_RESPONSES_MODEL,
      messages: [
        { role: "user", content: "Use the cached assistant prefill." },
        chatCompletionAssistantCacheControlMessage,
      ],
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        { role: "user", content: "Use the cached assistant prefill." },
        {
          role: "assistant",
          content: [
            {
              type: "text",
              text: "This assistant prefill should remain cacheable.",
              cache_control: { type: "ephemeral", ttl: "1h" },
            },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  chatCompletionsSystemCacheControlParam: {
    "chat-completions": {
      model: OPENAI_RESPONSES_MODEL,
      messages: [
        chatCompletionSystemCacheControlMessage,
        { role: "user", content: "Now summarize it." },
      ],
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      system: [
        {
          type: "text",
          text: chatCompletionCacheControlTextPart.text,
          cache_control: { type: "ephemeral", ttl: "1h" },
        },
      ],
      messages: [{ role: "user", content: "Now summarize it." }],
    },
    google: null,
    bedrock: null,
  },

  // === Reasoning Configuration ===

  reasoningSummaryParam: {
    "chat-completions": {
      model: OPENAI_RESPONSES_MODEL, // Must use reasoning model for reasoning_effort
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "medium",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "2+2" }],
      reasoning: {
        effort: "medium",
        summary: "detailed",
      },
    },
    anthropic: {
      model: ANTHROPIC_OPUS_MODEL,
      max_tokens: 16000,
      messages: [{ role: "user", content: "What is 2+2?" }],
      output_config: { effort: "medium" },
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "What is 2+2?" }] }],
      generationConfig: {
        thinkingConfig: {
          thinkingBudget: 10000,
          includeThoughts: true,
        },
      },
    },
    bedrock: null,
  },

  reasoningEffortLowParam: {
    "chat-completions": {
      model: OPENAI_RESPONSES_MODEL, // Must use reasoning model
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "low",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "What is 2+2?" }],
      reasoning: { effort: "low" },
    },
    anthropic: {
      model: ANTHROPIC_OPUS_MODEL,
      max_tokens: 16000,
      messages: [{ role: "user", content: "What is 2+2?" }],
      output_config: { effort: "low" },
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "What is 2+2?" }] }],
      generationConfig: {
        thinkingConfig: {
          thinkingBudget: 5000,
        },
      },
    },
    bedrock: null,
  },

  opus47AdaptiveThinkingReasoningEffortParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "medium",
      max_completion_tokens: 4096,
    },
    responses: null,
    anthropic: {
      model: "claude-opus-4-7",
      max_tokens: 4096,
      messages: [{ role: "user", content: "What is 2+2?" }],
      thinking: { type: "adaptive" },
      output_config: { effort: "medium" },
    },
    google: null,
    bedrock: null,
  },

  reasoningEffortMinimalParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "minimal",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "What is 2+2?" }],
      reasoning: { effort: "minimal" },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  reasoningEffortNoneParam: {
    "chat-completions": {
      model: OPENAI_REASONING_NONE_MODEL,
      messages: [{ role: "user", content: "What is 2+2?" }],
      reasoning_effort: "none",
    },
    responses: {
      model: OPENAI_REASONING_NONE_MODEL,
      input: [{ role: "user", content: "What is 2+2?" }],
      reasoning: { effort: "none" },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  reasoningEffortMaxClampsToGpt5NanoParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_OPUS_MODEL,
      max_tokens: 16000,
      messages: [{ role: "user", content: "What is 2+2?" }],
      output_config: { effort: "max" },
    },
    google: null,
    bedrock: null,
  },

  responsesReasoningEffortMaxParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "What is 2+2?" }],
      reasoning: { effort: "max" },
    },
    anthropic: {
      model: ANTHROPIC_OPUS_MODEL,
      max_tokens: 16000,
      messages: [{ role: "user", content: "What is 2+2?" }],
      output_config: { effort: "max" },
    },
    google: null,
    bedrock: null,
  },

  anthropicOpus5AdaptiveThinkingMaxEffortParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "What is 2+2?" }],
      reasoning: { effort: "max" },
    },
    anthropic: {
      model: "claude-opus-5",
      max_tokens: 65536,
      stream: true,
      messages: [{ role: "user", content: "What is 2+2?" }],
      thinking: { type: "adaptive" },
      output_config: { effort: "max" },
    },
    google: null,
    bedrock: null,
  },

  anthropicOpus5DisabledThinkingHighEffortParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: "claude-opus-5",
      max_tokens: 4096,
      messages: [{ role: "user", content: "What is 2+2?" }],
      thinking: { type: "disabled" },
      output_config: { effort: "high" },
    },
    google: null,
    bedrock: null,
  },

  responsesInputFileUrlParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: [
            {
              type: "input_text",
              text: "Analyze the letter and summarize the key points.",
            },
            {
              type: "input_file",
              file_url: "https://www.berkshirehathaway.com/letters/2024ltr.pdf",
            },
          ],
        },
      ],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  chatCompletionsUrlBackedAudioFileParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "Transcribe this audio clip.",
            },
            {
              type: "file",
              file: {
                filename: "sample-3s.mp3",
                file_data: "https://samplelib.com/mp3/sample-3s.mp3",
              },
            },
          ],
        },
      ],
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  chatCompletionsUrlBackedVideoFileParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "Describe this video clip.",
            },
            {
              type: "file",
              file: {
                filename: "sample-5s.mp4",
                file_data:
                  "https://samplelib.com/lib/preview/mp4/sample-5s.mp4",
              },
            },
          ],
        },
      ],
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  chatCompletionsGcsBackedVideoFileParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "Describe this video clip.",
            },
            {
              type: "file",
              file: {
                filename: "sample-200mb.mp4",
                file_data: "gs://lingua-test-bucket/sample-200mb.mp4",
              },
            },
          ],
        },
      ],
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
    "vertex-google": {
      model: VERTEX_GOOGLE_MODEL,
      contents: [
        {
          role: "user",
          parts: [
            { text: "Describe this video clip." },
            {
              fileData: {
                fileUri: "gs://lingua-test-bucket/sample-200mb.mp4",
                mimeType: "video/mp4",
              },
            },
          ],
        },
      ],
    },
  },

  responsesFunctionCallOutputWithoutThoughtSignatureParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: [
            {
              type: "input_text",
              text: "What databases exist in the connected MongoDB instance? Use the list_databases tool.",
            },
          ],
        },
        {
          type: "function_call",
          call_id: "6k7x6c84",
          name: "list_databases",
          arguments: "{}",
        },
        {
          type: "function_call_output",
          call_id: "6k7x6c84",
          output:
            '[{"type":"text","text":"{\\"databases\\":[\\"admin\\",\\"config\\",\\"local\\"]}"}]',
        },
      ],
      tools: [
        {
          type: "function",
          name: "list_databases",
          description: "List databases in the connected MongoDB instance.",
          parameters: {
            type: "object",
            properties: {},
            additionalProperties: false,
          },
          strict: false,
        },
      ],
      tool_choice: "auto",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  responsesAdditionalToolsParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "Use any additional tools made available later.",
        },
        {
          type: "additional_tools",
          role: "developer",
          tools: [
            {
              type: "function",
              name: "lookup_policy",
              description: "Look up an internal policy by slug.",
              parameters: {
                type: "object",
                properties: {
                  slug: { type: "string" },
                },
                required: ["slug"],
                additionalProperties: false,
              },
              strict: true,
            },
          ],
        },
      ],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  responsesAdditionalToolsMultipleToolsParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content:
            "Use the extra policy and note tools once the developer makes them available.",
        },
        {
          id: "at_payload_123",
          type: "additional_tools",
          role: "developer",
          tools: [
            {
              type: "function",
              name: "lookup_policy",
              description: "Look up an internal policy by slug.",
              parameters: {
                type: "object",
                properties: {
                  slug: { type: "string" },
                },
                required: ["slug"],
                additionalProperties: false,
              },
              strict: true,
            },
            {
              type: "custom",
              name: "write_release_note",
              description: "Draft a release note in plain text.",
              format: {
                type: "text",
              },
            },
          ],
        },
      ],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  responsesCustomToolCallStreamingParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content:
            "Call write_release_note with a non-empty plain-text release note about the streaming custom-tool fix. Do not provide a normal response.",
        },
      ],
      tools: [
        {
          type: "custom",
          name: "write_release_note",
          description: "Draft a release note in plain text.",
          format: { type: "text" },
        },
      ],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  responsesGpt56ReasoningMaxProContextParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: "Review this rollout checklist for the highest-risk issue.",
      reasoning: {
        effort: "max",
        mode: "pro",
        context: "all_turns",
      },
      text: {
        verbosity: "high",
      },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  responsesGpt56PersistedReasoningParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      store: false,
      include: ["reasoning.encrypted_content"],
      reasoning: {
        effort: "low",
        context: "all_turns",
      },
      input: [
        {
          role: "user",
          content:
            "Summarize the deployment risk and preserve reasoning state.",
        },
      ],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  responsesGpt56PromptCacheOptionsParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: [
            {
              type: "input_text",
              text: "Use the stable policy prefix when answering.",
              prompt_cache_breakpoint: { mode: "explicit" },
            },
          ],
        },
      ],
      prompt_cache_options: {
        mode: "explicit",
        ttl: "30m",
      },
      prompt_cache_retention: "24h",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  responsesProgrammaticToolCallingToolsParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: "Compare inventory and demand for sku_123.",
      tools: [
        {
          type: "function",
          name: "get_inventory",
          description: "Return inventory details for a SKU.",
          parameters: {
            type: "object",
            properties: {
              sku: { type: "string" },
            },
            required: ["sku"],
            additionalProperties: false,
          },
          strict: true,
          output_schema: {
            type: "object",
            properties: {
              sku: { type: "string" },
              available_units: { type: "number" },
            },
            required: ["sku", "available_units"],
            additionalProperties: false,
          },
          allowed_callers: ["programmatic"],
        },
        {
          type: "custom",
          name: "write_short_note",
          description: "Write a compact plain-text note.",
          format: { type: "text" },
          allowed_callers: ["direct", "programmatic"],
        },
        {
          type: "programmatic_tool_calling",
        },
      ],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  imageUrlMimeTypeFallbackParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "text",
              text: "Describe this image.",
            },
            {
              type: "image_url",
              image_url: {
                url: "https://t3.ftcdn.net/jpg/02/36/99/22/360_F_236992283_sNOxCVQeFLd5pdqaKGh8DRGMZy7P4XKm.jpg",
              },
            },
          ],
        },
      ],
    },
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [
            { text: "Describe this image." },
            {
              fileData: {
                fileUri:
                  "https://t3.ftcdn.net/jpg/02/36/99/22/360_F_236992283_sNOxCVQeFLd5pdqaKGh8DRGMZy7P4XKm.jpg",
                mimeType: "image/jpeg",
              },
            },
          ],
        },
      ],
    },
    bedrock: null,
  },

  // === Text Response Configuration ===

  textFormatJsonObjectParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: 'Return {"status": "ok"} as JSON.' }],
      response_format: { type: "json_object" },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Return JSON with a=1" }],
      text: {
        format: {
          type: "json_object",
        },
      },
    },
    anthropic: null,
    google: {
      contents: [
        { role: "user", parts: [{ text: 'Return {"status": "ok"} as JSON.' }] },
      ],
      generationConfig: {
        responseMimeType: "application/json",
      },
    },
    bedrock: null,
  },

  textFormatJsonSchemaParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "Extract: John is 25.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "person_info",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Name: John, Age: 25" }],
      text: {
        format: {
          type: "json_schema",
          name: "person_info",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_config: {
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
        },
      },
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Extract: John is 25." }] }],
      generationConfig: {
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            name: { type: "string" },
            age: { type: "number" },
          },
          required: ["name", "age"],
        },
      },
    },
    bedrock: null,
  },

  textFormatJsonSchemaMissingRequiredPropertyParam: {
    "chat-completions": {
      model: OPENAI_MINI_REASONING_MODEL,
      messages: [
        {
          role: "user",
          content: "Return an answer and short reasoning.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "structured_response",
          schema: {
            type: "object",
            properties: {
              answer: { type: "string" },
              reasoning: { type: "string" },
            },
            required: ["answer"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_MINI_REASONING_MODEL,
      input: [
        {
          role: "user",
          content: "Return an answer and short reasoning.",
        },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "structured_response",
          schema: {
            type: "object",
            properties: {
              answer: { type: "string" },
              reasoning: { type: "string" },
            },
            required: ["answer"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  textFormatJsonSchemaWithDescriptionParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "Extract: John is 25.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "person_info",
          description: "Extract person information from text",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Name: John, Age: 25" }],
      text: {
        format: {
          type: "json_schema",
          name: "person_info",
          description: "Extract person information from text",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_config: {
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
        },
      },
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Extract: John is 25." }] }],
      generationConfig: {
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            name: { type: "string" },
            age: { type: "number" },
          },
          required: ["name", "age"],
        },
      },
    },
    bedrock: null,
  },

  textFormatJsonSchemaNullableUnionTypeGpt54NanoParam: {
    "chat-completions": {
      model: "gpt-5.4-nano",
      messages: [
        {
          role: "user",
          content: "Classify the query and return JSON.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "query_result",
          strict: true,
          schema: {
            type: "object",
            properties: {
              explanation: {
                type: "string",
              },
              filter: {
                type: ["string", "null"],
              },
              match: {
                type: "boolean",
              },
            },
            required: ["explanation", "filter", "match"],
            additionalProperties: false,
          },
        },
      },
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  googleResponseSchemaPropertyOrderingParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: "Return JSON with keys gateway and score." }],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseSchema: {
          type: Type.OBJECT,
          properties: {
            gateway: { type: Type.STRING },
            score: { type: Type.INTEGER },
          },
          required: ["gateway", "score"],
          propertyOrdering: ["gateway", "score"],
        },
      },
    },
    bedrock: null,
  },

  googleThinkingJsonSchemaParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_MODEL,
      contents: [
        {
          role: "user",
          parts: [
            {
              text: [
                "Analyze the caption and return a compact JSON summary.",
                "Caption: A creator compares two microphone setups, explains why the cheaper lavalier performs better outdoors, and asks viewers to comment with their setup.",
                'Return exactly {"topic": "...", "recommendation": "...", "confidence": 0.0}.',
              ].join("\n"),
            },
          ],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 2048,
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            topic: { type: "string" },
            recommendation: { type: "string" },
            confidence: { type: "number" },
          },
          required: ["topic", "recommendation", "confidence"],
        },
        thinkingConfig: {
          thinkingBudget: 1024,
          includeThoughts: true,
        },
      },
    },
    bedrock: null,
  },

  jsonSchemaPrefixItemsParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        { role: "user", content: 'Return {"tuple": ["gateway", 7]} as JSON.' },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "tuple_response",
          schema: {
            type: "object",
            properties: {
              tuple: {
                type: "array",
                prefixItems: [{ type: "string" }, { type: "integer" }],
                items: { anyOf: [{ type: "string" }, { type: "integer" }] },
                minItems: 2,
                maxItems: 2,
              },
            },
            required: ["tuple"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        { role: "user", content: 'Return {"tuple": ["gateway", 7]} as JSON.' },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "tuple_response",
          schema: {
            type: "object",
            properties: {
              tuple: {
                type: "array",
                prefixItems: [{ type: "string" }, { type: "integer" }],
                items: { anyOf: [{ type: "string" }, { type: "integer" }] },
                minItems: 2,
                maxItems: 2,
              },
            },
            required: ["tuple"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: 'Return {"tuple": ["gateway", 7]} as JSON.' }],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            tuple: {
              type: "array",
              prefixItems: [{ type: "string" }, { type: "integer" }],
              items: { anyOf: [{ type: "string" }, { type: "integer" }] },
              minItems: 2,
              maxItems: 2,
            },
          },
          required: ["tuple"],
        },
      },
    },
    bedrock: null,
  },

  jsonSchemaFormatParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "Return JSON with an ISO 8601 timestamp in created_at.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "timestamp_response",
          schema: {
            type: "object",
            properties: {
              created_at: { type: "string", format: "date-time" },
            },
            required: ["created_at"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "Return JSON with an ISO 8601 timestamp in created_at.",
        },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "timestamp_response",
          schema: {
            type: "object",
            properties: {
              created_at: { type: "string", format: "date-time" },
            },
            required: ["created_at"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        {
          role: "user",
          content: "Return JSON with an ISO 8601 timestamp in created_at.",
        },
      ],
      output_config: {
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              created_at: { type: "string", format: "date-time" },
            },
            required: ["created_at"],
            additionalProperties: false,
          },
        },
      },
    },
    google: {
      contents: [
        {
          role: "user",
          parts: [
            {
              text: "Return JSON with an ISO 8601 timestamp in created_at.",
            },
          ],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            created_at: { type: "string", format: "date-time" },
          },
          required: ["created_at"],
        },
      },
    },
    bedrock: null,
  },

  jsonSchemaMinMaxItemsParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "Return JSON with tags as an array of 2 to 3 strings.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "tag_list",
          schema: {
            type: "object",
            properties: {
              tags: {
                type: "array",
                items: { type: "string" },
                minItems: 2,
                maxItems: 3,
              },
            },
            required: ["tags"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "Return JSON with tags as an array of 2 to 3 strings.",
        },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "tag_list",
          schema: {
            type: "object",
            properties: {
              tags: {
                type: "array",
                items: { type: "string" },
                minItems: 2,
                maxItems: 3,
              },
            },
            required: ["tags"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [
            { text: "Return JSON with tags as an array of 2 to 3 strings." },
          ],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            tags: {
              type: "array",
              items: { type: "string" },
              minItems: 2,
              maxItems: 3,
            },
          },
          required: ["tags"],
        },
      },
    },
    bedrock: null,
  },

  jsonSchemaMinimumMaximumParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "Return JSON with score as an integer from 0 to 10.",
        },
      ],
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "bounded_score",
          schema: {
            type: "object",
            properties: {
              score: {
                type: "integer",
                minimum: 0,
                maximum: 10,
              },
            },
            required: ["score"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "Return JSON with score as an integer from 0 to 10.",
        },
      ],
      text: {
        format: {
          type: "json_schema",
          name: "bounded_score",
          schema: {
            type: "object",
            properties: {
              score: {
                type: "integer",
                minimum: 0,
                maximum: 10,
              },
            },
            required: ["score"],
            additionalProperties: false,
          },
          strict: true,
        },
      },
    },
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [
            { text: "Return JSON with score as an integer from 0 to 10." },
          ],
        },
      ],
      generationConfig: {
        temperature: 0,
        maxOutputTokens: 128,
        responseMimeType: "application/json",
        responseJsonSchema: {
          type: "object",
          properties: {
            score: {
              type: "integer",
              minimum: 0,
              maximum: 10,
            },
          },
          required: ["score"],
        },
      },
    },
    bedrock: null,
  },

  textFormatTextParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say hello." }],
      response_format: { type: "text" },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Say hello." }],
      text: { format: { type: "text" } },
    },
    anthropic: null, // text is default, no explicit param needed
    google: null,
    bedrock: null,
  },

  // === Tool Configuration ===

  webSearchToolParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Latest OpenAI news" }],
      tools: [{ type: "web_search_preview" }],
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Latest OpenAI news" }],
      tools: [
        {
          type: "web_search_20250305",
          name: "web_search",
        },
      ],
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Latest OpenAI news" }] }],
      tools: [{ googleSearch: {} }],
    },
    bedrock: null,
  },

  // Provider-hosted code execution tools are not lossless analogues:
  // Responses code_interpreter is Python/container based, Anthropic bash is a
  // shell tool, and Google codeExecution is a Google-specific execution tool.
  codeInterpreterToolParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [
        {
          role: "user",
          content: "Execute Python code to generate a random number",
        },
      ],
      tools: [{ type: "code_interpreter", container: { type: "auto" } }],
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Run Python" }],
      tools: [
        {
          type: "bash_20250124",
          name: "bash",
        },
      ],
    },
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: "Execute Python code to generate a random number" }],
        },
      ],
      tools: [{ codeExecution: {} }],
    },
    bedrock: null,
  },

  toolChoiceRequiredParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Tokyo weather" }],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get weather",
            strict: true,
            parameters: {
              type: "object",
              properties: { location: { type: "string" } },
              required: ["location"],
              additionalProperties: false,
            },
          },
        },
      ],
      tool_choice: { type: "function", function: { name: "get_weather" } },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Tokyo weather" }],
      tools: [
        {
          type: "function",
          name: "get_weather",
          description: "Get weather",
          strict: true,
          parameters: {
            type: "object",
            properties: {
              location: { type: "string" },
            },
            required: ["location"],
            additionalProperties: false,
          },
        },
      ],
      tool_choice: { type: "function", name: "get_weather" },
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Tokyo weather" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: {
              location: { type: "string" },
            },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "tool", name: "get_weather" },
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Tokyo weather" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: {
                  location: { type: Type.STRING },
                },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.ANY,
          allowedFunctionNames: ["get_weather"],
        },
      },
    },
    bedrock: null,
  },

  toolChoiceRequiredWithReasoningParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Tokyo weather" }],
      reasoning_effort: "medium",
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get weather",
            strict: true,
            parameters: {
              type: "object",
              properties: { location: { type: "string" } },
              required: ["location"],
              additionalProperties: false,
            },
          },
        },
      ],
      tool_choice: { type: "function", function: { name: "get_weather" } },
    },
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_GEMINI_3_MODEL,
      contents: [{ role: "user", parts: [{ text: "Tokyo weather" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: {
                  location: { type: Type.STRING },
                },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.ANY,
          allowedFunctionNames: ["get_weather"],
        },
      },
      generationConfig: {
        thinkingConfig: {
          thinkingLevel: ThinkingLevel.MEDIUM,
          includeThoughts: true,
        },
      },
    },
    bedrock: null,
  },

  // Reproduces: "Function tools with reasoning_effort are not supported for
  // gpt-5.4-mini in /v1/chat/completions. Please use /v1/responses instead."
  // The router should detect reasoning_effort + function tools and forward to
  // the responses endpoint rather than passing through to chat/completions.
  functionToolsWithReasoningEffortParam: {
    "chat-completions": {
      model: OPENAI_MINI_REASONING_MODEL,
      messages: [{ role: "user", content: "Tokyo weather" }],
      reasoning_effort: "medium",
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get weather",
            strict: true,
            parameters: {
              type: "object",
              properties: { location: { type: "string" } },
              required: ["location"],
              additionalProperties: false,
            },
          },
        },
      ],
    },
    responses: {
      model: OPENAI_MINI_REASONING_MODEL,
      input: [{ role: "user", content: "Tokyo weather" }],
      reasoning: { effort: "medium" },
      tools: [
        {
          type: "function",
          name: "get_weather",
          description: "Get weather",
          strict: true,
          parameters: {
            type: "object",
            properties: { location: { type: "string" } },
            required: ["location"],
            additionalProperties: false,
          },
        },
      ],
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  googleToolCallThoughtSignatureReplayParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        {
          role: "user",
          content: "List the collections in the mydb database.",
        },
        googleToolCallThoughtSignatureReplayAssistantMessage,
        {
          role: "tool",
          tool_call_id: "call_123",
          content: JSON.stringify(["movies", "users"]),
        },
      ],
      tools: [
        {
          type: "function",
          function: {
            name: "list_collections",
            description: "List the collections in a MongoDB database.",
            parameters: {
              type: "object",
              properties: {
                database: { type: "string" },
              },
              required: ["database"],
            },
          },
        },
      ],
      tool_choice: "auto",
    },
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: "List the collections in the mydb database." }],
        },
        {
          role: "model",
          parts: [
            {
              functionCall: {
                name: "list_collections",
                args: { database: "mydb" },
              },
              thoughtSignature: "dGhvdWdodF9zaWduYXR1cmVfMTIz",
            },
          ],
        },
        {
          role: "user",
          parts: [
            {
              functionResponse: {
                name: "list_collections",
                response: { output: ["movies", "users"] },
              },
            },
          ],
        },
      ],
      tools: [
        {
          functionDeclarations: [
            {
              name: "list_collections",
              description: "List the collections in a MongoDB database.",
              parameters: {
                type: Type.OBJECT,
                properties: {
                  database: { type: Type.STRING },
                },
                required: ["database"],
              },
            },
          ],
        },
      ],
    },
    bedrock: null,
  },

  parallelToolCallsDisabledParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Weather in NYC and LA?" }],
      tools: [
        {
          type: "function",
          function: {
            name: "get_weather",
            description: "Get weather",
            strict: true,
            parameters: {
              type: "object",
              properties: { location: { type: "string" } },
              required: ["location"],
              additionalProperties: false,
            },
          },
        },
      ],
      parallel_tool_calls: false,
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "NYC and LA weather" }],
      tools: [
        {
          type: "function",
          name: "get_weather",
          description: "Get weather",
          strict: true,
          parameters: {
            type: "object",
            properties: {
              location: { type: "string" },
            },
            required: ["location"],
            additionalProperties: false,
          },
        },
      ],
      parallel_tool_calls: false,
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "NYC and LA weather" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: {
              location: { type: "string" },
            },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "auto", disable_parallel_tool_use: true },
    },
    google: null,
    bedrock: null,
  },

  // === Context & State Management ===

  instructionsParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [
        { role: "system", content: "Always say ok." },
        { role: "user", content: "Hi" },
      ],
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      instructions: "Reply with OK",
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Hi" }],
      system: "Say OK",
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Hi" }] }],
      systemInstruction: { parts: [{ text: "Always say ok." }] },
    },
    bedrock: null,
  },

  truncationAutoParam: {
    "chat-completions": null,
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      truncation: "auto",
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  storeDisabledParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      store: false,
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      store: false,
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Caching & Performance ===

  serviceTierParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      service_tier: "default",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      service_tier: "default",
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say ok." }],
      service_tier: "auto",
    },
    google: null,
    bedrock: null,
  },

  promptCacheKeyParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      prompt_cache_key: "user-123-ml-explanation",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      prompt_cache_key: "test-key",
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      system: [
        {
          type: "text",
          text: "Be helpful.",
          cache_control: { type: "ephemeral" },
        },
      ],
      messages: [{ role: "user", content: "Say ok." }],
    },
    google: null,
    bedrock: null,
  },

  // === Metadata & Identification ===

  metadataParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      store: true,
      metadata: {
        request_id: "req-12345",
        user_tier: "premium",
        experiment: "control",
      },
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      metadata: { key: "value" },
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say ok." }],
      metadata: { user_id: "user-12345" },
    },
    google: null,
    bedrock: null,
  },

  safetyIdentifierParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      safety_identifier: "hashed-user-id-abc123",
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Hi" }],
      safety_identifier: "test-user",
    },
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say ok." }],
      metadata: { user_id: "hashed-user-id-abc123" },
    },
    google: null,
    bedrock: null,
  },

  // === Sampling Parameters (require non-reasoning model) ===

  temperatureParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say hi." }],
      temperature: 0.7,
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
      temperature: 0.7,
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Say hi." }] }],
      generationConfig: {
        temperature: 0.7,
      },
    },
    bedrock: null,
  },

  fableTemperatureParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say hi." }],
      temperature: 0.7,
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_FABLE_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
    },
    google: null,
    bedrock: null,
  },

  bedrockAnthropicOpus48SamplingParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say hi." }],
      temperature: 0.7,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: {
      modelId: "global.anthropic.claude-opus-4-8",
      messages: [
        {
          role: "user",
          content: [{ text: "Say hi." }],
        },
      ],
    },
  },

  topPParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say hi." }],
      top_p: 0.9,
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
      top_p: 0.9,
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Say hi." }] }],
      generationConfig: {
        topP: 0.9,
      },
    },
    bedrock: null,
  },

  topPReasoningModelParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say hi." }],
      top_p: 0.9,
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Say hi." }],
      top_p: 0.9,
    },
    anthropic: null,
    google: null,
    bedrock: null,
  },

  frequencyPenaltyParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      frequency_penalty: 0.5,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  presencePenaltyParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      presence_penalty: 0.5,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  logprobsParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "What is 2 + 2?" }],
      logprobs: true,
      top_logprobs: 2,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Output Control ===

  nMultipleCompletionsParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say a word." }],
      n: 2,
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  stopSequencesParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Count from 1 to 20." }],
      stop: ["10", "ten"],
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Count to 20." }],
      stop_sequences: ["10", "ten"],
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Count from 1 to 20." }] }],
      generationConfig: {
        stopSequences: ["10", "ten"],
      },
    },
    bedrock: null,
  },

  maxCompletionTokensParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Say ok." }],
      max_completion_tokens: 500,
    },
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 500,
      messages: [{ role: "user", content: "Say ok." }],
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Say ok." }] }],
      generationConfig: {
        maxOutputTokens: 500,
      },
    },
    bedrock: null,
  },

  // === Advanced Parameters ===

  predictionParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [
        {
          role: "user",
          content:
            "Update this function to add error handling:\n\nfunction divide(a, b) {\n  return a / b;\n}",
        },
      ],
      prediction: {
        type: "content",
        content:
          "function divide(a, b) {\n  if (b === 0) {\n    throw new Error('Cannot divide by zero');\n  }\n  return a / b;\n}",
      },
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  seedParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Pick a number." }],
      seed: 12345,
    },
    responses: null,
    anthropic: null,
    google: {
      contents: [{ role: "user", parts: [{ text: "Pick a number." }] }],
      generationConfig: {
        seed: 12345,
      },
    },
    bedrock: null,
  },

  logitBiasParam: {
    "chat-completions": {
      model: OPENAI_NON_REASONING_MODEL,
      messages: [{ role: "user", content: "Say hello." }],
      logit_bias: { "15339": -100 },
    },
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  // === Anthropic-Specific Parameters ===

  topKParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
      top_k: 40,
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Say hi." }] }],
      generationConfig: {
        topK: 40,
      },
    },
    bedrock: null,
  },

  googleOpenAIModelTopKParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [{ text: "Write a short sentence about API gateways." }],
        },
      ],
      generationConfig: {
        topK: 1,
        maxOutputTokens: 1024,
      },
    },
    bedrock: null,
  },

  streamParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Say hi." }],
      stream: true,
    },
    google: null,
    bedrock: null,
  },

  textEditorToolParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Edit file." }],
      tools: [{ type: "text_editor_20250124", name: "str_replace_editor" }],
    },
    google: null,
    bedrock: null,
  },

  textEditorToolV2Param: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Edit file." }],
      tools: [
        { type: "text_editor_20250429", name: "str_replace_based_edit_tool" },
      ],
    },
    google: null,
    bedrock: null,
  },

  textEditorToolV3Param: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Edit file." }],
      tools: [
        {
          type: "text_editor_20250728",
          name: "str_replace_based_edit_tool",
          max_characters: 10000,
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  webSearchToolAdvancedParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "AI news" }],
      tools: [
        {
          type: "web_search_20250305",
          name: "web_search",
          allowed_domains: ["wikipedia.org", "arxiv.org"],
          max_uses: 3,
        },
      ],
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "AI news" }] }],
      tools: [
        {
          googleSearch: {
            timeRangeFilter: {
              startTime: "2025-01-01T00:00:00Z",
              endTime: "2025-01-03T00:00:00Z",
            },
          },
        },
      ],
    },
    bedrock: null,
  },

  webSearchUserLocationParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Local food" }],
      tools: [
        {
          type: "web_search_20250305",
          name: "web_search",
          user_location: {
            type: "approximate",
            city: "San Francisco",
            region: "California",
            country: "US",
            timezone: "America/Los_Angeles",
          },
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  toolChoiceAutoParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Weather?" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: { location: { type: "string" } },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "auto" },
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Weather?" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: { location: { type: Type.STRING } },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.AUTO,
        },
      },
    },
    bedrock: null,
  },

  toolChoiceAnyParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Weather?" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: { location: { type: "string" } },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "any" },
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Weather?" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: { location: { type: Type.STRING } },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.ANY,
        },
      },
    },
    bedrock: null,
  },

  toolChoiceNoneParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Weather?" }],
      tools: [
        {
          name: "get_weather",
          description: "Get weather",
          input_schema: {
            type: "object",
            properties: { location: { type: "string" } },
            required: ["location"],
          },
        },
      ],
      tool_choice: { type: "none" },
    },
    google: {
      contents: [{ role: "user", parts: [{ text: "Weather?" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: { location: { type: Type.STRING } },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.NONE,
        },
      },
    },
    bedrock: null,
  },

  cacheControl5mParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      system: [
        {
          type: "text",
          text: "Be helpful.",
          cache_control: { type: "ephemeral", ttl: "5m" },
        },
      ],
      messages: [{ role: "user", content: "Hi" }],
    },
    google: null,
    bedrock: null,
  },

  cacheControl1hParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      system: [
        {
          type: "text",
          text: "Be helpful.",
          cache_control: { type: "ephemeral", ttl: "1h" },
        },
      ],
      messages: [{ role: "user", content: "Hi" }],
    },
    google: null,
    bedrock: null,
  },

  imageContentParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "image",
              source: {
                type: "base64",
                media_type: "image/png",
                data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
              },
            },
            { type: "text", text: "Describe." },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  documentContentParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [
        {
          role: "user",
          content: [
            {
              type: "document",
              source: {
                type: "text",
                media_type: "text/plain",
                data: "Sample text.",
              },
              title: "Doc",
            },
            { type: "text", text: "Summarize." },
          ],
        },
      ],
    },
    google: null,
    bedrock: null,
  },

  thinkingDisabledParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "2+2?" }],
      thinking: { type: "disabled" },
    },
    google: null,
    bedrock: null,
  },

  // === Output Config (structured output) ===

  outputFormatJsonSchemaParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_format: {
        type: "json_schema",
        schema: {
          type: "object",
          properties: {
            name: { type: "string" },
            age: { type: "number" },
          },
          required: ["name", "age"],
          additionalProperties: false,
        },
      },
    },
    google: null,
    bedrock: null,
  },

  outputConfigJsonSchemaParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_MODEL,
      max_tokens: 1024,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_config: {
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
        },
      },
    },
    google: null,
    bedrock: null,
  },

  outputConfigEffortWithJsonSchemaParam: {
    "chat-completions": null,
    responses: null,
    anthropic: {
      model: ANTHROPIC_OPUS_MODEL,
      max_tokens: 16000,
      messages: [{ role: "user", content: "Extract: John is 25." }],
      output_config: {
        effort: "medium",
        format: {
          type: "json_schema",
          schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "number" },
            },
            required: ["name", "age"],
            additionalProperties: false,
          },
        },
      },
    },
    google: null,
    bedrock: null,
  },

  // === Google-Specific Parameters ===

  thinkingLevelParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_GEMINI_3_MODEL,
      contents: [
        { role: "user", parts: [{ text: "Solve this complex problem." }] },
      ],
      generationConfig: {
        thinkingConfig: {
          thinkingLevel: ThinkingLevel.HIGH,
          includeThoughts: true,
        },
      },
    },
    bedrock: null,
  },

  toolModeValidatedParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [{ role: "user", parts: [{ text: "Weather in Tokyo?" }] }],
      tools: [
        {
          functionDeclarations: [
            {
              name: "get_weather",
              description: "Get weather",
              parameters: {
                type: Type.OBJECT,
                properties: { location: { type: Type.STRING } },
                required: ["location"],
              },
            },
          ],
        },
      ],
      toolConfig: {
        functionCallingConfig: {
          mode: FunctionCallingConfigMode.VALIDATED,
        },
      },
    },
    bedrock: null,
  },

  thoughtSignatureParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: null,
    bedrock: null,
  },

  urlContextToolParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [
        {
          role: "user",
          parts: [
            {
              text: "Summarize https://ai.google.dev/gemini-api/docs/url-context and highlight the key constraints.",
            },
          ],
        },
      ],
      tools: [{ urlContext: {} }],
    },
    bedrock: null,
  },

  responseModalitiesAudioParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_TTS_MODEL,
      contents: [
        {
          role: "user",
          parts: [{ text: "Say hello in a warm, concise voice." }],
        },
      ],
      generationConfig: {
        responseModalities: [Modality.AUDIO],
      },
    },
    bedrock: null,
  },

  speechConfigParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_TTS_MODEL,
      contents: [
        {
          role: "user",
          parts: [
            {
              text: 'Generate audio speaking exactly this text: "Hello."',
            },
          ],
        },
      ],
      generationConfig: {
        responseModalities: [Modality.AUDIO],
        speechConfig: {
          voiceConfig: {
            prebuiltVoiceConfig: {
              voiceName: "Kore",
            },
          },
        },
      },
    },
    bedrock: null,
  },

  imageConfigParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      model: GOOGLE_IMAGE_MODEL,
      contents: [
        { role: "user", parts: [{ text: "Generate a tiny red dot." }] },
      ],
      generationConfig: {
        responseModalities: [Modality.IMAGE],
        imageConfig: {
          aspectRatio: "1:1",
        },
      },
    },
    bedrock: null,
  },

  mediaResolutionParam: {
    "chat-completions": null,
    responses: null,
    anthropic: null,
    google: {
      contents: [
        { role: "user", parts: [{ text: "Describe this image briefly." }] },
      ],
      generationConfig: {
        mediaResolution: MediaResolution.MEDIA_RESOLUTION_LOW,
      },
    },
    bedrock: null,
  },

  googleToolSchemaNumericInt64Param: (() => {
    const indexNameSchema: Record<string, unknown> = {
      type: Type.STRING,
      minLength: 1,
      maxLength: 128,
    };
    const tagsSchema: Record<string, unknown> = {
      type: Type.ARRAY,
      items: { type: Type.STRING },
      minItems: 1,
      maxItems: 3,
    };

    const testCase: TestCase = {
      "chat-completions": null,
      responses: null,
      anthropic: null,
      google: {
        model: GOOGLE_MODEL,
        contents: [
          { role: "user", parts: [{ text: "Validate tool schema bounds." }] },
        ],
        tools: [
          {
            functionDeclarations: [
              {
                name: "validate_bounds",
                description: "Validate bounded string and array inputs.",
                parameters: {
                  type: Type.OBJECT,
                  properties: {
                    index_name: indexNameSchema,
                    tags: tagsSchema,
                  },
                  required: ["index_name", "tags"],
                },
              },
            ],
          },
        ],
      },
      bedrock: null,
    };
    return testCase;
  })(),

  exclusiveMinimumToolParam: {
    "chat-completions": {
      model: OPENAI_CHAT_COMPLETIONS_MODEL,
      messages: [{ role: "user", content: "Configure the LLM." }],
      tools: [
        {
          type: "function",
          function: {
            name: "configure_llm",
            description: "Configure LLM generation parameters",
            parameters: {
              type: "object",
              properties: {
                max_tokens: {
                  type: "number",
                  exclusiveMinimum: 0,
                  description: "Maximum number of tokens to generate",
                },
              },
              required: ["max_tokens"],
              additionalProperties: false,
            },
          },
        },
      ],
    },
    responses: {
      model: OPENAI_RESPONSES_MODEL,
      input: [{ role: "user", content: "Configure the LLM." }],
      tools: [
        {
          type: "function",
          name: "configure_llm",
          description: "Configure LLM generation parameters",
          parameters: {
            type: "object",
            properties: {
              max_tokens: {
                type: "number",
                exclusiveMinimum: 0,
                description: "Maximum number of tokens to generate",
              },
            },
            required: ["max_tokens"],
            additionalProperties: false,
          },
          strict: false,
        },
      ],
    },
    anthropic: null,
    google: (() => {
      // Assigned to a variable first so TypeScript applies structural (not
      // excess-property) checking when it lands in Record<string, Schema>.
      // exclusiveMinimum is not in Gemini's Schema type but IS passed here
      // deliberately to capture the resulting 400 INVALID_ARGUMENT error.
      const maxTokensSchema = {
        type: Type.NUMBER,
        exclusiveMinimum: 0,
        description: "Maximum number of tokens to generate",
      };
      return {
        model: GOOGLE_MODEL,
        contents: [{ role: "user", parts: [{ text: "Configure the LLM." }] }],
        tools: [
          {
            functionDeclarations: [
              {
                name: "configure_llm",
                description: "Configure LLM generation parameters",
                parameters: {
                  type: Type.OBJECT,
                  properties: { max_tokens: maxTokensSchema },
                  required: ["max_tokens"],
                },
              },
            ],
          },
        ],
      };
    })(),
    bedrock: null,
  },

  anthropicMessageWithSystemMessage: (() => {
    const testCase: TestCase = {
      "chat-completions": null,
      responses: null,
      anthropic: {
        model: "claude-opus-4-8",
        max_tokens: 32_000,
        system: [
          {
            type: "text",
            text: "You are running inside Claude Code.",
          },
          {
            type: "text",
            text: "Preserve the user's coding instructions.",
            cache_control: { type: "ephemeral" },
          },
        ],
        messages: [
          {
            role: "user",
            content: [
              {
                type: "text",
                text: "hello world",
                cache_control: { type: "ephemeral" },
              },
            ],
          },
          {
            role: "system",
            content: "Only use the exact tools provided by Claude Code.",
          },
        ],
        tools: [
          {
            name: "Read",
            description: "Reads a file from the local filesystem.",
            input_schema: {
              type: "object",
              properties: {
                file_path: {
                  type: "string",
                },
              },
              required: ["file_path"],
              additionalProperties: false,
            },
          },
        ],
        thinking: {
          type: "adaptive",
        },
        output_config: {
          effort: "high",
        },
      },
      google: null,
      bedrock: null,
    };

    return testCase;
  })(),
};
